import { create } from 'zustand'
import * as tauri from '../lib/tauri'
import { useConfigStore } from './config.store'
import { useConnectionStore } from './connection.store'
import { useThemeStore } from './theme.store'

interface AppStore {
  initialized: boolean
  isElevated: boolean | null
  binariesOk: boolean | null
  timestampsOk: boolean | null

  initialize: () => Promise<void>
  setBinariesOk: (ok: boolean) => void
}

export const useAppStore = create<AppStore>((set, get) => ({
  initialized: false,
  isElevated: null,
  binariesOk: null,
  timestampsOk: null,

  initialize: async () => {
    if (get().initialized) return

    useThemeStore.getState().initTheme()
    useConnectionStore.getState().initTrayListener()

    const elevated = await tauri.isElevated()
    set({ isElevated: elevated })

    if (elevated) {
      const timestamps = await tauri.checkTcpTimestamps()
      set({ timestampsOk: timestamps })

      if (!timestamps) {
        await tauri.enableTcpTimestamps()
        useConnectionStore.getState().addLog('TCP timestamps включены')
        set({ timestampsOk: true })
      }

      await useConfigStore.getState().load()
      const binaries = await tauri.verifyBinaries()
      set({ binariesOk: binaries })

      const orphanPid = await tauri.checkAndRecoverOrphan()
      if (orphanPid) {
        useConnectionStore.getState().setRecovered(true)
        useConnectionStore.getState().addLog(`Обнаружен запущенный процесс winws.exe (PID: ${orphanPid})`)
      } else {
        await useConnectionStore.getState().checkStatus()
      }
    }

    set({ initialized: true })
  },

  setBinariesOk: (ok) => set({ binariesOk: ok }),
}))
