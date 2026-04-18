import type { AppHealthSnapshot } from '@/lib/types'
import { List } from 'lucide-react'
import { createElement } from 'react'
import { toast } from 'sonner'
import { create } from 'zustand'
import * as tauri from '../lib/tauri'
import { useConfigStore } from './config.store'
import { useConnectionStore } from './connection.store'
import { useDownloadStore } from './download.store'
import { useThemeStore } from './theme.store'
import { useUpdaterStore } from './updater.store'

let shutdownCleanupRegistered = false
const LIST_REFRESH_TOAST_DELAY_MS = 600
const LIST_DISPLAY_NAMES: Record<string, string> = {
  'zapret-hosts-user-exclude.txt': 'список исключений',
  'zapret-ip-user.txt': 'список заблокированных адресов',
  'zapret-hosts-google.txt': 'список Google/YouTube',
}

function formatUpdatedListToast(filename: string) {
  const label = LIST_DISPLAY_NAMES[filename] ?? filename
  return `Обновлён ${label}`
}

interface AppStore {
  initialized: boolean
  initializing: boolean
  mainPageVisited: boolean
  mainTerminalTimeOffset: number
  isElevated: boolean | null
  healthSnapshot: AppHealthSnapshot | null
  binariesOk: boolean | null
  timestampsOk: boolean | null
  missingCriticalFiles: string[]
  availableUpdates: string[]
  configMissing: boolean
  initializePromise: Promise<void> | null
  filesWatcherCleanup: (() => void) | null
  backgroundChecksCleanup: (() => void) | null
  refreshVersion: number

  initialize: () => Promise<void>
  refreshLocalState: () => Promise<void>
  refreshRemoteState: () => Promise<void>
  applyHealthSnapshot: (snapshot: AppHealthSnapshot) => void
  setBinariesOk: (ok: boolean) => void
  setMissingCriticalFiles: (files: string[]) => void
  setAvailableUpdates: (files: string[]) => void
  setConfigMissing: (missing: boolean) => void
  setMainPageVisited: (visited: boolean) => void
  teardownFilesWatcher: () => void
}

export const useAppStore = create<AppStore>((set, get) => ({
  initialized: false,
  initializing: false,
  mainPageVisited: false,
  mainTerminalTimeOffset: Math.random() * 100,
  isElevated: null,
  healthSnapshot: null,
  binariesOk: null,
  timestampsOk: null,
  missingCriticalFiles: [],
  availableUpdates: [],
  configMissing: false,
  initializePromise: null,
  filesWatcherCleanup: null,
  backgroundChecksCleanup: null,
  refreshVersion: 0,
  teardownFilesWatcher: () => {
    const cleanup = get().filesWatcherCleanup
    if (cleanup) {
      cleanup()
      set({ filesWatcherCleanup: null })
    }
    const backgroundCleanup = get().backgroundChecksCleanup
    if (backgroundCleanup) {
      backgroundCleanup()
      set({ backgroundChecksCleanup: null })
    }
  },
  refreshLocalState: async () => {
    const version = get().refreshVersion + 1
    set({ refreshVersion: version })

    const snapshot = await tauri.getAppHealthSnapshot(false)
    if (get().refreshVersion !== version) {
      return
    }

    get().applyHealthSnapshot(snapshot)
  },
  refreshRemoteState: async () => {
    const version = get().refreshVersion + 1
    set({ refreshVersion: version })

    let listRefreshToastId: string | number | null = null
    const listRefreshToastTimer = window.setTimeout(() => {
      listRefreshToastId = toast.loading('Обновляю списки...')
    }, LIST_REFRESH_TOAST_DELAY_MS)

    try {
      const updatedLists = await tauri.refreshListsIfStale()
      if (updatedLists.length > 0) {
        useConnectionStore.getState().addLog(
          `Списки обновлены автоматически: ${updatedLists.join(', ')}`,
        )
        if (listRefreshToastId) {
          toast.dismiss(listRefreshToastId)
        }
        for (const filename of updatedLists) {
          toast(formatUpdatedListToast(filename), {
            icon: createElement(List, { className: 'h-4 w-4 text-success' }),
          })
        }
      }
      else if (listRefreshToastId) {
        toast.dismiss(listRefreshToastId)
      }
    }
    catch (e) {
      useConnectionStore.getState().addLog(`Не удалось автоматически обновить списки: ${e}`)
      const message = `Не удалось обновить списки: ${e instanceof Error ? e.message : String(e)}`
      if (listRefreshToastId) {
        toast.error(message, { id: listRefreshToastId })
      }
      else {
        toast.error(message)
      }
    }
    finally {
      window.clearTimeout(listRefreshToastTimer)
    }

    const snapshot = await tauri.getAppHealthSnapshot(true)
    if (get().refreshVersion !== version) {
      return
    }

    get().applyHealthSnapshot(snapshot)
    if (!snapshot.available_updates_checked) {
      useConnectionStore.getState().addLog('Не удалось получить удалённые обновления файлов, использую локальное состояние')
    }
  },

  initialize: async () => {
    if (get().initialized)
      return

    if (get().initializePromise)
      return get().initializePromise!

    const initializePromise = (async () => {
      set({ initializing: true })

      if (!shutdownCleanupRegistered && typeof window !== 'undefined') {
        shutdownCleanupRegistered = true
        window.addEventListener('beforeunload', () => {
          get().teardownFilesWatcher()
          useConnectionStore.getState().teardownTrayListener()
          useDownloadStore.getState().cleanup()
          useUpdaterStore.getState().cleanup()
        })
      }

      useConnectionStore.getState().addLog('Запускаю инициализацию приложения')
      useThemeStore.getState().initTheme()
      useConnectionStore.getState().initTrayListener()
      await useDownloadStore.getState().initListeners()

      const elevated = await tauri.isElevated()
      set({ isElevated: elevated })
      useConnectionStore.getState().addLog(
        elevated
          ? 'Приложение запущено с правами администратора'
          : 'Приложение запущено без прав администратора',
      )

      if (elevated) {
        const timestamps = await tauri.checkTcpTimestamps()
        set({ timestampsOk: timestamps })
        useConnectionStore.getState().addLog(
          timestamps
            ? 'TCP timestamps уже включены'
            : 'TCP timestamps отключены, включаю автоматически',
        )

        if (!timestamps) {
          await tauri.enableTcpTimestamps()
          useConnectionStore.getState().addLog('TCP timestamps включены')
          set({ timestampsOk: true })
        }

        useConnectionStore.getState().addLog('Проверяю и восстанавливаю управляемые файлы')
        try {
          const ensured = await tauri.ensureManagedFiles()
          if (ensured.config_restored) {
            useConnectionStore.getState().addLog('config.json был автоматически восстановлен из дефолтного конфига')
          }
          if (ensured.config_reloaded) {
            useConnectionStore.getState().addLog('Конфигурация была нормализована и повторно загружена с диска')
          }
          if (ensured.restored_files.length > 0) {
            useConnectionStore.getState().addLog(`Автоматически восстановлены файлы: ${ensured.restored_files.join(', ')}`)
          }
          if (ensured.unrecoverable_filters.length > 0) {
            useConnectionStore.getState().addLog(`Не удалось автоматически восстановить фильтры: ${ensured.unrecoverable_filters.join(', ')}`)
            toast.error(`Не удалось автоматически восстановить фильтры: ${ensured.unrecoverable_filters.join(', ')}`)
          }
        }
        catch (e) {
          console.error('ensureManagedFiles failed during startup:', e)
          useConnectionStore.getState().addLog(`Удалённые файлы сейчас недоступны, продолжаю с локальным состоянием: ${e}`)
        }

        // ensureManagedFiles() may partially repair runtime state before failing,
        // so we always re-read config from disk to normalize the resulting state.
        useConnectionStore.getState().addLog('Загружаю конфигурацию')
        await useConfigStore.getState().load()
        if (useConfigStore.getState().config) {
          set({ configMissing: false })
          useConnectionStore.getState().addLog('Конфигурация загружена')
        }
        else {
          set({ configMissing: true })
          useConnectionStore.getState().addLog('Не удалось загрузить конфигурацию')
          throw new Error('Config not loaded')
        }

        const snapshot = await tauri.getAppHealthSnapshot(false)
        get().applyHealthSnapshot(snapshot)
        useConnectionStore.getState().addLog(
          snapshot.binaries_ok
            ? 'Файлы приложения и списки прошли проверку целостности'
            : 'Файлы приложения или списки отсутствуют либо повреждены',
        )

        if (!get().backgroundChecksCleanup && typeof window !== 'undefined') {
          const intervalId = window.setInterval(() => {
            get().refreshRemoteState().catch((e) => {
              useConnectionStore.getState().addLog(`Фоновая проверка файлов завершилась ошибкой: ${e}`)
            })
          }, 3 * 60 * 60 * 1000)

          set({
            backgroundChecksCleanup: () => {
              window.clearInterval(intervalId)
            },
          })
        }

        get().refreshRemoteState().catch((e) => {
          useConnectionStore.getState().addLog(`Первичная проверка обновлений завершилась ошибкой: ${e}`)
        })

        if (!get().filesWatcherCleanup) {
          const offHealthChanged = tauri.onFilesHealthChanged(({ binaries_ok, lists_changed, config_missing, config_restored, config_reloaded, restored_files, unrecoverable_filters }) => {
            const previousState = get().binariesOk
            set({ configMissing: config_missing })

            if (config_restored || config_reloaded) {
              void useConfigStore.getState().reload().catch((error) => {
                useConnectionStore.getState().addLog(`Не удалось перезагрузить конфигурацию после watcher-события: ${error}`)
              })
            }

            if (config_restored) {
              useConnectionStore.getState().addLog('Watcher восстановил config.json из дефолтного конфига')
            }

            if (config_reloaded && !config_restored) {
              useConnectionStore.getState().addLog('Watcher перезагрузил config.json после внешнего изменения')
            }

            if (restored_files.length > 0) {
              useConnectionStore.getState().addLog(`Watcher восстановил файлы: ${restored_files.join(', ')}`)
              toast.success(
                restored_files.length === 1
                  ? `Восстановлен системный файл: ${restored_files[0]}`
                  : `Восстановлены системные файлы: ${restored_files.slice(0, 4).join(', ')}${restored_files.length > 4 ? '…' : ''}`,
              )
            }

            if (unrecoverable_filters.length > 0) {
              const message = `Не удалось автоматически восстановить фильтры: ${unrecoverable_filters.join(', ')}`
              useConnectionStore.getState().addLog(message)
              toast.error(message)
            }

            get().refreshLocalState().catch((error) => {
              useConnectionStore.getState().addLog(`Не удалось обновить состояние файлов приложения: ${error}`)
            })

            if (previousState !== binaries_ok) {
              useConnectionStore.getState().addLog(
                binaries_ok
                  ? 'Локальные управляемые файлы снова в порядке'
                  : 'Обнаружено локальное изменение: один или несколько управляемых файлов отсутствуют либо повреждены',
              )
            }

            if (lists_changed) {
              useConnectionStore.getState().addLog('Обнаружено локальное изменение файлов списков')
            }

            if (config_missing) {
              useConnectionStore.getState().addLog('config.json отсутствует и не был автоматически восстановлен')
            }
          })

          const offWatchError = tauri.onFilesHealthWatchError((message) => {
            useConnectionStore.getState().addLog(message)
          })

          set({
            filesWatcherCleanup: () => {
              offHealthChanged()
              offWatchError()
            },
          })
        }

        const orphanPid = await tauri.checkAndRecoverOrphan()
        if (orphanPid) {
          useConnectionStore.getState().setRecovered(true)
          useConnectionStore.getState().addLog(`Обнаружен запущенный процесс winws.exe (PID: ${orphanPid})`)
        }
        else {
          await useConnectionStore.getState().checkStatus()

          const currentConfig = useConfigStore.getState().config
          const launchedFromAutostart = await tauri.wasLaunchedFromAutostart()
          if (launchedFromAutostart && currentConfig?.connectOnAutostart) {
            useConnectionStore.getState().addLog('Приложение запущено из автозагрузки, запускаю подключение автоматически')
            await useConnectionStore.getState().connect()
          }
        }
      }

      if (useConfigStore.getState().config) {
        try {
          await useUpdaterStore.getState().init()
        }
        catch (error) {
          useConnectionStore.getState().addLog(`Не удалось инициализировать автообновления приложения: ${error}`)
        }
      }

      useConnectionStore.getState().addLog('Инициализация приложения завершена')
      set({ initialized: true, initializing: false, initializePromise: null })
    })().catch((error) => {
      set({ initializing: false, initializePromise: null })
      get().teardownFilesWatcher()
      useConnectionStore.getState().teardownTrayListener()
      useDownloadStore.getState().cleanup()
      useUpdaterStore.getState().cleanup()
      useConnectionStore.getState().addLog(`Ошибка инициализации приложения: ${error}`)
      throw error
    })

    set({ initializePromise })
    return initializePromise
  },

  applyHealthSnapshot: (snapshot) => {
    set((state) => {
      const nextAvailableUpdates = snapshot.available_updates_checked
        ? snapshot.available_updates
        : state.availableUpdates

      return {
        healthSnapshot: {
          ...snapshot,
          available_updates: nextAvailableUpdates,
        },
        binariesOk: snapshot.binaries_ok,
        missingCriticalFiles: snapshot.missing_critical_files,
        availableUpdates: nextAvailableUpdates,
        configMissing: snapshot.config_missing,
      }
    })
  },
  setBinariesOk: ok => set({ binariesOk: ok }),
  setMissingCriticalFiles: files => set({ missingCriticalFiles: files }),
  setAvailableUpdates: files => set({ availableUpdates: files }),
  setConfigMissing: missing => set({ configMissing: missing }),
  setMainPageVisited: visited => set({ mainPageVisited: visited }),
}))
