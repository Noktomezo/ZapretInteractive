import type { DownloadProgress } from '@/lib/types'
import { listen } from '@tauri-apps/api/event'
import { toast } from 'sonner'
import { create } from 'zustand'
import * as tauri from '../lib/tauri'
import { useAppStore } from './app.store'

interface DownloadStore {
  isDownloading: boolean
  progress: DownloadProgress | null
  listenersInitialized: boolean
  unlistenFns: Array<() => void>
  setDownloading: (downloading: boolean) => void
  setProgress: (progress: DownloadProgress | null) => void
  reset: () => void
  initListeners: () => Promise<void>
  cleanup: () => void
}

export const useDownloadStore = create<DownloadStore>(set => ({
  isDownloading: false,
  progress: null,
  listenersInitialized: false,
  unlistenFns: [],
  setDownloading: isDownloading => set({ isDownloading }),
  setProgress: progress => set({ progress }),
  reset: () => set({ isDownloading: false, progress: null }),
  initListeners: async () => {
    if (useDownloadStore.getState().listenersInitialized)
      return
    try {
      const unlistenStart = await listen('download-start', () => {
        useDownloadStore.getState().setDownloading(true)
      })

      const unlistenProgress = await listen<DownloadProgress>('download-progress', (event) => {
        useDownloadStore.getState().setProgress(event.payload)
      })

      const unlistenComplete = await listen('download-complete', async () => {
        try {
          const version = useAppStore.getState().refreshVersion + 1
          useAppStore.setState({ refreshVersion: version })
          const binaries = await tauri.verifyBinaries()
          const missingCriticalFiles = await tauri.getMissingCriticalFiles()
          if (useAppStore.getState().refreshVersion !== version) {
            return
          }
          useAppStore.getState().setBinariesOk(binaries)
          useAppStore.getState().setMissingCriticalFiles(missingCriticalFiles)
          if (binaries) {
            try {
              const availableUpdates = await tauri.getAvailableUpdates()
              if (useAppStore.getState().refreshVersion !== version) {
                return
              }
              useAppStore.getState().setAvailableUpdates(availableUpdates)
            }
            catch (e) {
              if (useAppStore.getState().refreshVersion !== version) {
                return
              }
              useAppStore.getState().setAvailableUpdates([])
              toast.error(`Ошибка проверки обновлений файлов: ${e}`)
            }
          }
          else {
            useAppStore.getState().setAvailableUpdates([])
          }
        }
        catch (e) {
          useAppStore.getState().setBinariesOk(false)
          useAppStore.getState().setMissingCriticalFiles([])
          useAppStore.getState().setAvailableUpdates([])
          toast.error(`Ошибка проверки файлов: ${e}`)
        }
        finally {
          useDownloadStore.getState().reset()
        }
      })

      const unlistenError = await listen<string>('download-error', (event) => {
        console.error('Download error:', event.payload)
        useDownloadStore.getState().reset()
      })

      set({
        listenersInitialized: true,
        unlistenFns: [unlistenStart, unlistenProgress, unlistenComplete, unlistenError],
      })
    }
    catch (e) {
      useDownloadStore.getState().cleanup()
      throw e
    }
  },
  cleanup: () => {
    for (const unlisten of useDownloadStore.getState().unlistenFns)
      unlisten()

    set({
      unlistenFns: [],
      listenersInitialized: false,
      isDownloading: false,
      progress: null,
    })
  },
}))
