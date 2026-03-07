import { listen } from '@tauri-apps/api/event'
import { toast } from 'sonner'
import { create } from 'zustand'
import * as tauri from '../lib/tauri'
import { useAppStore } from './app.store'

interface DownloadProgress {
  current: number
  total: number
  filename: string
  phase: 'binaries' | 'fake' | 'lists' | 'filters'
}

interface DownloadStore {
  isDownloading: boolean
  progress: DownloadProgress | null
  listenersInitialized: boolean
  setDownloading: (downloading: boolean) => void
  setProgress: (progress: DownloadProgress | null) => void
  reset: () => void
  initListeners: () => void
}

export const useDownloadStore = create<DownloadStore>(set => ({
  isDownloading: false,
  progress: null,
  listenersInitialized: false,
  setDownloading: isDownloading => set({ isDownloading }),
  setProgress: progress => set({ progress }),
  reset: () => set({ isDownloading: false, progress: null }),
  initListeners: () => {
    if (useDownloadStore.getState().listenersInitialized)
      return

    set({ listenersInitialized: true })

    void listen('download-start', () => {
      useDownloadStore.getState().setDownloading(true)
    })

    void listen<DownloadProgress>('download-progress', (event) => {
      useDownloadStore.getState().setProgress(event.payload)
    })

    void listen('download-complete', async () => {
      useDownloadStore.getState().reset()
      try {
        const binaries = await tauri.verifyBinaries()
        useAppStore.getState().setBinariesOk(binaries)
      }
      catch (e) {
        toast.error(`Ошибка проверки файлов: ${e}`)
      }
    })

    void listen<string>('download-error', (event) => {
      console.error('Download error:', event.payload)
      useDownloadStore.getState().reset()
    })
  },
}))
