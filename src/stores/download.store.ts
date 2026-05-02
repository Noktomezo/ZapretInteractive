import type { DownloadProgress } from '@/lib/types'
import { listen } from '@tauri-apps/api/event'
import { toast } from 'sonner'
import { create } from 'zustand'
import { useConnectionStore } from './connection.store'

export type DownloadCompleteHandler = () => Promise<void>

interface DownloadStore {
  isDownloading: boolean
  progress: DownloadProgress | null
  listenersInitialized: boolean
  unlistenFns: Array<() => void>
  setDownloading: (downloading: boolean) => void
  setProgress: (progress: DownloadProgress | null) => void
  reset: () => void
  initListeners: (onDownloadComplete?: DownloadCompleteHandler) => Promise<void>
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
  initListeners: async (onDownloadComplete) => {
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
          await onDownloadComplete?.()
        }
        catch (e) {
          useConnectionStore.getState().addLog(`Не удалось обновить локальное состояние файлов после загрузки: ${e}`)
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
