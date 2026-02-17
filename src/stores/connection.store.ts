import { create } from 'zustand'
import * as tauri from '../lib/tauri'
import { buildStrategyCommand, buildFiltersCommand } from '../lib/strategy'
import { useConfigStore } from './config.store'

type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'disconnecting' | 'error'

interface ConnectionStore {
  status: ConnectionStatus
  pid: number | null
  logs: string[]
  error: string | null
  recovered: boolean

  checkStatus: () => Promise<void>
  connect: () => Promise<void>
  disconnect: () => Promise<void>
  addLog: (log: string) => void
  clearLogs: () => void
  setError: (error: string | null) => void
  setRecovered: (recovered: boolean) => void
}

export const useConnectionStore = create<ConnectionStore>((set, get) => ({
  status: 'disconnected',
  pid: null,
  logs: [],
  error: null,
  recovered: false,

  checkStatus: async () => {
    try {
      const running = await tauri.isWinwsRunning()
      if (running) {
        const pid = await tauri.getRunningPid()
        set({ status: 'connected', pid })
      } else {
        set({ status: 'disconnected', pid: null })
      }
    } catch (e) {
      set({ status: 'disconnected', pid: null })
    }
  },

  connect: async () => {
    const config = useConfigStore.getState().config
    if (!config) {
      set({ error: 'Config not loaded', status: 'error' })
      return
    }

    set({ status: 'connecting', error: null })
    get().addLog('Starting connection...')

    try {
      const filtersDir = await tauri.getFiltersPath()
      
      const filtersCommand = buildFiltersCommand(config.filters, filtersDir)
      const strategyCommand = buildStrategyCommand(config)
      const processedStrategy = await tauri.resolvePlaceholders(strategyCommand, config.placeholders)
      
      let fullCommand = `winws.exe --wf-tcp=${config.global_ports.tcp} --wf-udp=${config.global_ports.udp}`
      if (filtersCommand) {
        fullCommand += ` ${filtersCommand.replace(/\n/g, ' ')}`
      }
      fullCommand += ` ${processedStrategy.replace(/\n/g, ' ')}`
      
      get().addLog(fullCommand)

      const combinedArgs = filtersCommand 
        ? `${filtersCommand}\n${processedStrategy}`
        : processedStrategy

      const pid = await tauri.startWinws(
        combinedArgs,
        config.global_ports.tcp,
        config.global_ports.udp
      )
      set({ status: 'connected', pid })
      get().addLog(`Connected with PID: ${pid}`)
    } catch (e) {
      set({ status: 'error', error: String(e) })
      get().addLog(`Error: ${e}`)
    }
  },

  disconnect: async () => {
    const { pid } = get()
    set({ status: 'disconnecting' })
    get().addLog('Disconnecting...')

    try {
      if (pid) {
        await tauri.stopWinws()
      } else {
        await tauri.killWindivertService()
      }
      set({ status: 'disconnected', pid: null })
      get().addLog('Disconnected')
    } catch (e) {
      set({ status: 'error', error: String(e) })
      get().addLog(`Error: ${e}`)
    }
  },

  addLog: (log) => {
    const timestamp = new Date().toLocaleTimeString()
    set((state) => ({ logs: [...state.logs, `[${timestamp}] ${log}`] }))
  },

  clearLogs: () => set({ logs: [] }),

  setError: (error) => set({ error }),

  setRecovered: (recovered) => {
    if (recovered) {
      set({ recovered: true, status: 'connected' })
    } else {
      set({ recovered: false })
    }
  },
}))