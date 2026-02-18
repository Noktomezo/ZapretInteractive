import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { AppConfig } from './types'

export const isElevated = (): Promise<boolean> => invoke('is_elevated')

export const ensureConfigDir = (): Promise<string> => invoke('ensure_config_dir')

export const loadConfig = (): Promise<AppConfig> => invoke('load_config')

export const saveConfig = (config: AppConfig): Promise<void> => invoke('save_config', { config })

export const resetConfig = (): Promise<AppConfig> => invoke('reset_config')

export const getZapretDirectory = (): Promise<string> => invoke('get_zapret_directory')

export const getHomeDirectory = (): Promise<string> => invoke<string>('get_zapret_directory').then((dir: string) => dir.replace(/\\.zapret$/, ''))

export const verifyBinaries = (): Promise<boolean> => invoke('verify_binaries')

export const downloadBinaries = async (): Promise<void> => invoke('download_binaries')

export const getWinwsPath = (): Promise<string> => invoke('get_winws_path')

export const startWinws = (args: string, tcpPorts: string, udpPorts: string): Promise<number> => invoke('start_winws', { args, tcpPorts, udpPorts })

export const stopWinws = (): Promise<void> => invoke('stop_winws')

export const isWinwsRunning = (): Promise<boolean> => invoke('is_winws_running')

export const killWindivertService = (): Promise<void> => invoke('kill_windivert_service')

export const getRunningPid = (): Promise<number> => invoke('get_running_pid')

export const checkAndRecoverOrphan = (): Promise<number | null> => invoke('check_and_recover_orphan')

export const openZapretDirectory = (): Promise<void> => invoke('open_zapret_directory')

export const getFiltersPath = (): Promise<string> => invoke('get_filters_path')

export const saveFilterFile = (filename: string, content: string): Promise<void> => 
  invoke('save_filter_file', { filename, content })

export const loadFilterFile = (filename: string): Promise<string> => 
  invoke('load_filter_file', { filename })

export const deleteFilterFile = (filename: string): Promise<void> => 
  invoke('delete_filter_file', { filename })

export const resolvePlaceholders = (content: string, placeholders: { name: string; path: string }[]): Promise<string> => 
  invoke('resolve_placeholders', { content, placeholders })

export const checkTcpTimestamps = (): Promise<boolean> => invoke('check_tcp_timestamps')

export const enableTcpTimestamps = (): Promise<void> => invoke('enable_tcp_timestamps')

export const setConnectedState = (connected: boolean): Promise<void> => invoke('set_connected_state', { connected })

export const onTrayConnectToggle = (callback: () => void): (() => void) => {
  let unlisten: (() => void) | null = null
  listen('tray-connect-toggle', () => callback()).then(fn => unlisten = fn)
  return () => unlisten?.()
}