import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { toast } from 'sonner'
import { create } from 'zustand'
import * as tauri from '@/lib/tauri'
import { useConfigStore } from './config.store'
import { useConnectionStore } from './connection.store'

const APP_UPDATE_CHECK_INTERVAL_MS = 5 * 60 * 1000
const APP_UPDATE_TIMEOUT_MS = 15_000

interface AvailableAppUpdate {
  version: string
  date?: string
  notes?: string
}

interface CheckForUpdatesOptions {
  silent?: boolean
  manual?: boolean
}

interface UpdaterStore {
  initialized: boolean
  checking: boolean
  downloading: boolean
  installing: boolean
  currentVersion: string | null
  availableUpdate: AvailableAppUpdate | null
  dismissedVersionThisSession: string | null
  lastCheckedAt: number | null
  lastCheckError: string | null
  downloadedBytes: number
  contentLength: number | null
  intervalCleanup: (() => void) | null

  init: () => Promise<void>
  syncAutoChecks: (enabled: boolean) => void
  checkForUpdates: (options?: CheckForUpdatesOptions) => Promise<void>
  installAvailableUpdate: () => Promise<void>
  dismissCurrentVersionUntilRestart: () => void
  clearDismissedVersionIfChanged: (nextVersion: string | null) => void
  cleanup: () => void
}

let pendingUpdateHandle: Update | null = null
let checkPromise: Promise<void> | null = null
let installPromise: Promise<void> | null = null
let configSubscriptionCleanup: (() => void) | null = null

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

function getUpdateSummary(update: Update): AvailableAppUpdate {
  return {
    version: update.version,
    date: update.date,
    notes: update.body,
  }
}

async function replacePendingUpdateHandle(nextUpdate: Update | null): Promise<void> {
  const previousUpdate = pendingUpdateHandle
  pendingUpdateHandle = nextUpdate
  if (previousUpdate && previousUpdate !== nextUpdate) {
    await previousUpdate.close().catch(() => {})
  }
}

export const useUpdaterStore = create<UpdaterStore>((set, get) => ({
  initialized: false,
  checking: false,
  downloading: false,
  installing: false,
  currentVersion: null,
  availableUpdate: null,
  dismissedVersionThisSession: null,
  lastCheckedAt: null,
  lastCheckError: null,
  downloadedBytes: 0,
  contentLength: null,
  intervalCleanup: null,

  init: async () => {
    if (get().initialized) {
      return
    }

    const currentVersion = await tauri.getAppVersion()

    if (!configSubscriptionCleanup) {
      configSubscriptionCleanup = useConfigStore.subscribe((state, previousState) => {
        const previousEnabled = previousState.config?.appAutoUpdatesEnabled ?? true
        const nextEnabled = state.config?.appAutoUpdatesEnabled ?? true
        if (previousEnabled !== nextEnabled && get().initialized) {
          get().syncAutoChecks(nextEnabled)
        }
      })
    }

    set({ initialized: true, currentVersion })
    get().syncAutoChecks(useConfigStore.getState().config?.appAutoUpdatesEnabled ?? true)
  },

  syncAutoChecks: (enabled) => {
    const cleanup = get().intervalCleanup
    if (cleanup) {
      cleanup()
      setIntervalCleanup(null)
    }

    if (!enabled || typeof window === 'undefined') {
      return
    }

    const intervalId = window.setInterval(() => {
      void get().checkForUpdates({ silent: true })
    }, APP_UPDATE_CHECK_INTERVAL_MS)

    setIntervalCleanup(() => {
      window.clearInterval(intervalId)
    })

    void get().checkForUpdates({ silent: true })
  },

  checkForUpdates: async ({ silent = false, manual = false } = {}) => {
    if (checkPromise) {
      return checkPromise
    }

    checkPromise = (async () => {
      set({ checking: true, lastCheckError: null })

      try {
        const currentVersion = await tauri.getAppVersion()
        const update = await check({ timeout: APP_UPDATE_TIMEOUT_MS })

        if (!update) {
          await replacePendingUpdateHandle(null)
          set({
            currentVersion,
            availableUpdate: null,
            checking: false,
            lastCheckedAt: Date.now(),
            lastCheckError: null,
            downloadedBytes: 0,
            contentLength: null,
          })

          if (manual) {
            toast.success('Новых версий не найдено')
          }
          return
        }

        await replacePendingUpdateHandle(update)
        const nextUpdate = getUpdateSummary(update)
        get().clearDismissedVersionIfChanged(nextUpdate.version)
        set({
          currentVersion,
          availableUpdate: nextUpdate,
          checking: false,
          lastCheckedAt: Date.now(),
          lastCheckError: null,
        })
        useConnectionStore.getState().addLog(`Найдено обновление приложения: ${currentVersion} -> ${nextUpdate.version}`)
      }
      catch (error) {
        const message = getErrorMessage(error)
        set({
          checking: false,
          lastCheckedAt: Date.now(),
          lastCheckError: message,
        })
        useConnectionStore.getState().addLog(`Ошибка проверки обновлений приложения: ${message}`)
        if (!silent || manual) {
          toast.error(`Не удалось проверить обновления приложения: ${message}`)
        }
      }
      finally {
        checkPromise = null
      }
    })()

    return checkPromise
  },

  installAvailableUpdate: async () => {
    if (installPromise) {
      return installPromise
    }

    const update = pendingUpdateHandle
    if (!update || !get().availableUpdate) {
      throw new Error('Нет доступного обновления приложения')
    }

    installPromise = (async () => {
      const toastId = toast.loading('Загружаю обновление приложения...')
      let downloadedBytes = 0

      set({
        downloading: true,
        installing: false,
        lastCheckError: null,
        downloadedBytes: 0,
        contentLength: null,
      })
      useConnectionStore.getState().addLog(`Начата загрузка обновления приложения до версии ${get().availableUpdate?.version}`)

      try {
        await update.downloadAndInstall((event) => {
          if (event.event === 'Started') {
            downloadedBytes = 0
            set({
              downloading: true,
              installing: false,
              downloadedBytes: 0,
              contentLength: event.data.contentLength ?? null,
            })
            return
          }

          if (event.event === 'Progress') {
            downloadedBytes += event.data.chunkLength
            const contentLength = get().contentLength
            set({ downloadedBytes })
            if (contentLength && contentLength > 0) {
              const percent = Math.min(100, Math.round((downloadedBytes / contentLength) * 100))
              toast.loading('Загружаю обновление приложения...', {
                id: toastId,
                description: `${percent}%`,
              })
            }
            return
          }

          set({
            downloading: false,
            installing: true,
          })
          toast.loading('Устанавливаю обновление приложения...', { id: toastId })
        }, { timeout: APP_UPDATE_TIMEOUT_MS })

        await replacePendingUpdateHandle(null)
        set({
          availableUpdate: null,
          dismissedVersionThisSession: null,
          downloading: false,
          installing: false,
          downloadedBytes: 0,
          contentLength: null,
          lastCheckError: null,
        })
        useConnectionStore.getState().addLog('Обновление приложения установлено')
        toast.success('Обновление установлено. Перезапускаю приложение...', { id: toastId })

        try {
          await relaunch()
        }
        catch (error) {
          const message = getErrorMessage(error)
          useConnectionStore.getState().addLog(`Обновление приложения установлено, но не удалось выполнить перезапуск: ${message}`)
          toast.error(`Обновление установлено, но перезапуск не удался: ${message}`, { id: toastId })
        }
      }
      catch (error) {
        const message = getErrorMessage(error)
        set({
          downloading: false,
          installing: false,
          lastCheckError: message,
        })
        useConnectionStore.getState().addLog(`Ошибка загрузки или установки обновления приложения: ${message}`)
        toast.error(`Не удалось установить обновление приложения: ${message}`, { id: toastId })
        throw error
      }
      finally {
        installPromise = null
      }
    })()

    return installPromise
  },

  dismissCurrentVersionUntilRestart: () => {
    const version = get().availableUpdate?.version
    if (!version) {
      return
    }

    set({ dismissedVersionThisSession: version })
    useConnectionStore.getState().addLog(`Пользователь отложил обновление приложения до рестарта: ${version}`)
  },

  clearDismissedVersionIfChanged: (nextVersion) => {
    const dismissedVersion = get().dismissedVersionThisSession
    if (dismissedVersion && dismissedVersion !== nextVersion) {
      set({ dismissedVersionThisSession: null })
    }
  },

  cleanup: () => {
    const cleanup = get().intervalCleanup
    if (cleanup) {
      cleanup()
    }
    set({ intervalCleanup: null })

    if (configSubscriptionCleanup) {
      configSubscriptionCleanup()
      configSubscriptionCleanup = null
    }

    void replacePendingUpdateHandle(null)
    set({
      initialized: false,
      checking: false,
      downloading: false,
      installing: false,
      currentVersion: null,
      availableUpdate: null,
      dismissedVersionThisSession: null,
      lastCheckedAt: null,
      lastCheckError: null,
      downloadedBytes: 0,
      contentLength: null,
      intervalCleanup: null,
    })
  },
}))

function setIntervalCleanup(cleanup: (() => void) | null) {
  useUpdaterStore.setState({ intervalCleanup: cleanup })
}
