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
    const collected: (() => void)[] = []
    try {
      collected.push(
        await listen('download-start', () => {
          useDownloadStore.getState().setDownloading(true)
        }),
      )
      collected.push(
        await listen<DownloadProgress>('download-progress', (event) => {
          useDownloadStore.getState().setProgress(event.payload)
        }),
      )
      collected.push(
        await listen('download-complete', async () => {
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
        }),
      )
      collected.push(
        await listen<string>('download-error', (event) => {
          console.error('Download error:', event.payload)
          useDownloadStore.getState().reset()
        }),
      )

      set({
        listenersInitialized: true,
        unlistenFns: collected,
      })
    }
    catch (e) {
      for (const unlisten of collected) {
        try {
          unlisten()
        }
        catch {}
      }
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
