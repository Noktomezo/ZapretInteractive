import { create } from 'zustand'
import { buildFiltersCommand, buildStrategyCommand } from '../lib/strategy'
import * as tauri from '../lib/tauri'
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
  toggle: () => Promise<void>
  addLog: (log: string) => void
  clearLogs: () => void
  setError: (error: string | null) => void
  setRecovered: (recovered: boolean) => void
  initTrayListener: () => () => void
}

async function updateTrayState(connected: boolean) {
  try {
    await tauri.setConnectedState(connected)
  }
  catch { }
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
        updateTrayState(true)
      }
      else {
        set({ status: 'disconnected', pid: null })
        updateTrayState(false)
      }
    }
    catch {
      set({ status: 'disconnected', pid: null })
      updateTrayState(false)
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
        config.global_ports.udp,
      )
      set({ status: 'connected', pid })
      updateTrayState(true)
      get().addLog(`Connected with PID: ${pid}`)
    }
    catch (e) {
      set({ status: 'error', error: String(e) })
      updateTrayState(false)
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
      }
      else {
        await tauri.killWindivertService()
      }
      set({ status: 'disconnected', pid: null })
      updateTrayState(false)
      get().addLog('Disconnected')
    }
    catch (e) {
      set({ status: 'error', error: String(e) })
      updateTrayState(false)
      get().addLog(`Error: ${e}`)
    }
  },

  toggle: async () => {
    const { status } = get()
    if (status === 'connected') {
      await get().disconnect()
    }
    else if (status === 'disconnected') {
      await get().connect()
    }
  },

  addLog: (log) => {
    const timestamp = new Date().toLocaleTimeString()
    set(state => ({ logs: [...state.logs, `[${timestamp}] ${log}`] }))
  },

  clearLogs: () => set({ logs: [] }),

  setError: error => set({ error }),

  setRecovered: (recovered) => {
    if (recovered) {
      set({ recovered: true, status: 'connected' })
      updateTrayState(true)
    }
    else {
      set({ recovered: false })
    }
  },

  initTrayListener: () => {
    return tauri.onTrayConnectToggle(() => {
      get().toggle()
    })
  },
}))
