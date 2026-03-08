import { toast } from 'sonner'
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
  initializePromise: Promise<void> | null
  filesWatcherCleanup: (() => void) | null

  initialize: () => Promise<void>
  setBinariesOk: (ok: boolean) => void
  teardownFilesWatcher: () => void
}

export const useAppStore = create<AppStore>((set, get) => ({
  initialized: false,
  initializing: false,
  isElevated: null,
  binariesOk: null,
  timestampsOk: null,
  initializePromise: null,
  filesWatcherCleanup: null,
  teardownFilesWatcher: () => {
    const cleanup = get().filesWatcherCleanup
    if (cleanup) {
      cleanup()
      set({ filesWatcherCleanup: null })
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

        useConnectionStore.getState().addLog('Загружаю конфигурацию')
        await useConfigStore.getState().load()
        if (useConfigStore.getState().config) {
          useConnectionStore.getState().addLog('Конфигурация загружена')
        }
        else {
          useConnectionStore.getState().addLog('Не удалось загрузить конфигурацию')
          throw new Error('Config not loaded')
        }

        const binaries = await tauri.verifyBinaries()
        set({ binariesOk: binaries })
        useConnectionStore.getState().addLog(
          binaries
            ? 'Файлы приложения и фильтры прошли проверку целостности'
            : 'Файлы приложения или фильтры отсутствуют либо повреждены',
        )

        try {
          const updatedLists = await tauri.refreshListsIfStale()
          if (updatedLists > 0) {
            useConnectionStore.getState().addLog(`Списки обновлены автоматически: ${updatedLists} файлов`)
          }
        }
        catch (e) {
          useConnectionStore.getState().addLog(`Не удалось автоматически обновить списки: ${e}`)
        }

        if (!get().filesWatcherCleanup) {
          const offHealthChanged = tauri.onFilesHealthChanged(({ binaries_ok, lists_changed }) => {
            const previousState = get().binariesOk
            set({ binariesOk: binaries_ok })

            if (previousState !== binaries_ok) {
              useConnectionStore.getState().addLog(
                binaries_ok
                  ? 'Локальные файлы приложения снова в порядке'
                  : 'Обнаружено локальное изменение: файлы приложения или фильтры отсутствуют либо повреждены',
              )
            }

            if (previousState !== false && binaries_ok === false) {
              toast.error('Обнаружено изменение локальных файлов. Часть файлов приложения или фильтров отсутствует либо повреждена.')
            }

            if (lists_changed) {
              useConnectionStore.getState().addLog('Обнаружено локальное изменение файлов списков')
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
}))
