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

    set({ listenersInitialized: true })

    const unlistenStart = await listen('download-start', () => {
      useDownloadStore.getState().setDownloading(true)
    })

    const unlistenProgress = await listen<DownloadProgress>('download-progress', (event) => {
      useDownloadStore.getState().setProgress(event.payload)
    })

    const unlistenComplete = await listen('download-complete', async () => {
      useDownloadStore.getState().reset()
      try {
        const binaries = await tauri.verifyBinaries()
        useAppStore.getState().setBinariesOk(binaries)
      }
      catch (e) {
        useAppStore.getState().setBinariesOk(false)
        toast.error(`Ошибка проверки файлов: ${e}`)
      }
    })

    const unlistenError = await listen<string>('download-error', (event) => {
      console.error('Download error:', event.payload)
      toast.error(`Ошибка загрузки файлов: ${event.payload}`)
      useDownloadStore.getState().reset()
    })

    set({
      unlistenFns: [unlistenStart, unlistenProgress, unlistenComplete, unlistenError],
    })
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
