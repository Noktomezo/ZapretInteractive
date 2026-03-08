import { create } from 'zustand'
import * as tauri from '../lib/tauri'
import { useConfigStore } from './config.store'
import { useConnectionStore } from './connection.store'
import { useDownloadStore } from './download.store'
import { useThemeStore } from './theme.store'

let shutdownCleanupRegistered = false

interface AppStore {
  initialized: boolean
  initializing: boolean
  isElevated: boolean | null
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
  refreshRemoteState: () => Promise<void>
  setBinariesOk: (ok: boolean) => void
  setMissingCriticalFiles: (files: string[]) => void
  setAvailableUpdates: (files: string[]) => void
  setConfigMissing: (missing: boolean) => void
  teardownFilesWatcher: () => void
}

export const useAppStore = create<AppStore>((set, get) => ({
  initialized: false,
  initializing: false,
  isElevated: null,
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
  refreshRemoteState: async () => {
    const version = get().refreshVersion + 1
    set({ refreshVersion: version })

    try {
      const updatedLists = await tauri.refreshListsIfStale()
      if (updatedLists > 0) {
        useConnectionStore.getState().addLog(`Списки обновлены автоматически: ${updatedLists} файлов`)
      }
    }
    catch (e) {
      useConnectionStore.getState().addLog(`Не удалось автоматически обновить списки: ${e}`)
    }

    const stillHealthy = await tauri.verifyBinaries()
    const missingFiles = await tauri.getMissingCriticalFiles()
    if (get().refreshVersion !== version) {
      return
    }

    if (stillHealthy) {
      try {
        const availableUpdates = await tauri.getAvailableUpdates()
        if (get().refreshVersion !== version) {
          return
        }
        set({ binariesOk: stillHealthy, missingCriticalFiles: missingFiles, availableUpdates })
      }
      catch (e) {
        if (get().refreshVersion !== version) {
          return
        }
        set({ binariesOk: stillHealthy, missingCriticalFiles: missingFiles, availableUpdates: [] })
        useConnectionStore.getState().addLog(`Не удалось проверить обновления файлов: ${e}`)
      }
    }
    else {
      set({ binariesOk: stillHealthy, missingCriticalFiles: missingFiles, availableUpdates: [] })
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
          useDownloadStore.getState().cleanup()
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

        const configExists = await tauri.configExists()
        useConnectionStore.getState().addLog('Загружаю конфигурацию')
        await useConfigStore.getState().load()
        if (useConfigStore.getState().config) {
          set({ configMissing: !configExists })
          useConnectionStore.getState().addLog('Конфигурация загружена')
          if (!configExists)
            useConnectionStore.getState().addLog('Файл config.json отсутствует, ожидаю подтверждения на восстановление дефолтного конфига')
        }
        else {
          set({ configMissing: true })
          useConnectionStore.getState().addLog('Не удалось загрузить конфигурацию')
          throw new Error('Config not loaded')
        }

        const binaries = await tauri.verifyBinaries()
        const missingCriticalFiles = await tauri.getMissingCriticalFiles()
        set({ binariesOk: binaries, missingCriticalFiles })
        useConnectionStore.getState().addLog(
          binaries
            ? 'Файлы приложения и списки прошли проверку целостности'
            : 'Файлы приложения или списки отсутствуют либо повреждены',
        )

        if (!get().backgroundChecksCleanup && typeof window !== 'undefined') {
          const intervalId = window.setInterval(() => {
            void get().refreshRemoteState()
          }, 3 * 60 * 60 * 1000)

          set({
            backgroundChecksCleanup: () => {
              window.clearInterval(intervalId)
            },
          })
        }

        void get().refreshRemoteState()

        if (!get().filesWatcherCleanup) {
          const offHealthChanged = tauri.onFilesHealthChanged(({ binaries_ok, lists_changed, config_missing }) => {
            const previousState = get().binariesOk
            const version = get().refreshVersion + 1
            set({ binariesOk: binaries_ok, configMissing: config_missing, refreshVersion: version })
            tauri.getMissingCriticalFiles().then((files) => {
              if (get().refreshVersion !== version) {
                return
              }
              set({ missingCriticalFiles: files })
            }).catch(console.error)
            if (binaries_ok) {
              tauri.getAvailableUpdates().then((files) => {
                if (get().refreshVersion !== version) {
                  return
                }
                set({ availableUpdates: files })
              }).catch(console.error)
            }
            else {
              set({ availableUpdates: [] })
            }

            if (previousState !== binaries_ok) {
              useConnectionStore.getState().addLog(
                binaries_ok
                  ? 'Локальные файлы приложения снова в порядке'
                  : 'Обнаружено локальное изменение: файлы приложения или списки отсутствуют либо повреждены',
              )
            }

            if (lists_changed) {
              useConnectionStore.getState().addLog('Обнаружено локальное изменение файлов списков')
            }

            if (config_missing) {
              useConnectionStore.getState().addLog('Обнаружено локальное удаление config.json')
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

      useConnectionStore.getState().addLog('Инициализация приложения завершена')
      set({ initialized: true, initializing: false, initializePromise: null })
    })().catch((error) => {
      set({ initializing: false, initializePromise: null })
      useConnectionStore.getState().addLog(`Ошибка инициализации приложения: ${error}`)
      throw error
    })

    set({ initializePromise })
    return initializePromise
  },

  setBinariesOk: ok => set({ binariesOk: ok }),
  setMissingCriticalFiles: files => set({ missingCriticalFiles: files }),
  setAvailableUpdates: files => set({ availableUpdates: files }),
  setConfigMissing: missing => set({ configMissing: missing }),
}))
