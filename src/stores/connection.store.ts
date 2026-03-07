import { create } from 'zustand'
import { buildFiltersCommand, buildFiltersCommandArray, buildStrategyCommand } from '../lib/strategy'
import * as tauri from '../lib/tauri'
import { useConfigStore } from './config.store'

type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'disconnecting' | 'error'

export interface LogEntry {
  timestamp: number
  message: string
}

interface ConnectionStore {
  status: ConnectionStatus
  pid: number | null
  logs: LogEntry[]
  error: string | null
  recovered: boolean

  updateTrayState: (connected: boolean) => Promise<void>
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

export const useConnectionStore = create<ConnectionStore>((set, get) => ({
  status: 'disconnected',
  pid: null,
  logs: [],
  error: null,
  recovered: false,

  updateTrayState: async (connected: boolean) => {
    try {
      await tauri.setConnectedState(connected)
    }
    catch (e) {
      console.error('Failed to update tray state:', connected, e)
      get().addLog(`Не удалось обновить состояние трея: ${e}`)
    }
  },

  checkStatus: async () => {
    get().addLog('Проверяю текущее состояние winws.exe')

    try {
      const running = await tauri.isWinwsRunning()
      if (running) {
        const pid = await tauri.getRunningPid()
        set({ status: 'connected', pid })
        get().addLog(`Найден активный процесс winws.exe (PID: ${pid})`)
        get().updateTrayState(true)
      }
      else {
        set({ status: 'disconnected', pid: null })
        get().addLog('Активный процесс winws.exe не найден')
        get().updateTrayState(false)
      }
    }
    catch (e) {
      set({ status: 'disconnected', pid: null })
      get().updateTrayState(false)
      get().addLog(`Ошибка проверки состояния процесса: ${e}`)
    }
  },

  connect: async () => {
    const config = useConfigStore.getState().config
    if (!config) {
      set({ error: 'Config not loaded', status: 'error' })
      get().addLog('Ошибка: конфигурация не загружена')
      return
    }

    set({ status: 'connecting', error: null })
    get().addLog('Начинаю подключение')
    get().addLog(`Режим списков: ${config.listMode}`)
    get().addLog(`Порты: TCP ${config.global_ports.tcp}, UDP ${config.global_ports.udp}`)

    try {
      get().addLog('Получаю каталог фильтров')
      const filtersDir = await tauri.getFiltersPath()

      const filtersCommand = buildFiltersCommand(config.filters, filtersDir)
      const filtersArgs = buildFiltersCommandArray(config.filters, filtersDir)
      const strategyCommand = buildStrategyCommand(config)
      get().addLog('Разрешаю плейсхолдеры стратегии')
      const processedStrategy = await tauri.resolvePlaceholders(strategyCommand, config.placeholders)

      let fullCommand = `winws.exe --wf-tcp=${config.global_ports.tcp} --wf-udp=${config.global_ports.udp}`
      if (filtersCommand) {
        fullCommand += ` ${filtersCommand.replace(/\n/g, ' ')}`
      }
      fullCommand += ` ${processedStrategy.replace(/\n/g, ' ')}`

      const strategyArgs = processedStrategy.split('\n').filter(Boolean)
      const allArgs = [...filtersArgs, ...strategyArgs]

      get().addLog(`Подготовлено аргументов запуска: ${allArgs.length}`)
      get().addLog(fullCommand)
      get().addLog('Запускаю winws.exe')

      const pid = await tauri.startWinws(
        allArgs,
        config.global_ports.tcp,
        config.global_ports.udp,
      )
      set({ status: 'connected', pid })
      get().updateTrayState(true)
      get().addLog(`Подключение установлено, PID: ${pid}`)
    }
    catch (e) {
      set({ status: 'error', error: String(e) })
      get().updateTrayState(false)
      get().addLog(`Ошибка подключения: ${e}`)
    }
  },

  disconnect: async () => {
    const { pid } = get()
    set({ status: 'disconnecting' })
    get().addLog('Начинаю отключение')

    try {
      if (pid) {
        get().addLog(`Останавливаю winws.exe (PID: ${pid})`)
        await tauri.stopWinws()
      }
      else {
        get().addLog('PID не найден, выполняю очистку службы WinDivert')
        await tauri.killWindivertService()
      }
      set({ status: 'disconnected', pid: null })
      get().updateTrayState(false)
      get().addLog('Подключение остановлено')
    }
    catch (e) {
      set({ status: 'error', error: String(e) })
      get().updateTrayState(false)
      get().addLog(`Ошибка отключения: ${e}`)
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

  addLog: (message) => {
    set(state => ({ logs: [...state.logs, { timestamp: Date.now(), message }] }))
  },

  clearLogs: () => set({ logs: [] }),

  setError: error => set({ error }),

  setRecovered: (recovered) => {
    if (recovered) {
      set({ recovered: true, status: 'connected' })
      get().addLog('Восстановлено состояние уже запущенного подключения')
      get().updateTrayState(true)
    }
    else {
      set({ recovered: false })
    }
  },

  initTrayListener: () => {
    get().addLog('Подписка на события трея инициализирована')
    return tauri.onTrayConnectToggle(() => {
      get().addLog('Получена команда переключения из трея')
      get().toggle()
    })
  },
}))
