import type { AppConfig, AppHealthSnapshot, EnsureManagedFilesResult, ListMode, WindowMaterial } from './types'
import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface FileHealthChangedPayload {
  binaries_ok: boolean
  lists_changed: boolean
  config_missing: boolean
  config_restored: boolean
  config_reloaded: boolean
  restored_files: string[]
  unrecoverable_filters: string[]
}

export const isElevated = (): Promise<boolean> => invoke('is_elevated')
export const ensureConfigDir = (): Promise<string> => invoke('ensure_config_dir')
export const loadConfig = (): Promise<AppConfig> => invoke('load_config')
export const saveConfig = (config: AppConfig): Promise<void> => invoke('save_config', { config })
export const resetConfig = (): Promise<AppConfig> => invoke('reset_config')
export const configExists = (): Promise<boolean> => invoke('config_exists')
export const getResourcesDirectory = (): Promise<string> => invoke('get_resources_directory')
export const verifyBinaries = (): Promise<boolean> => invoke('verify_binaries')
export const getMissingCriticalFiles = (): Promise<string[]> => invoke('get_missing_critical_files')
export const getAppHealthSnapshot = (forceRemoteUpdates = false): Promise<AppHealthSnapshot> => invoke('get_app_health_snapshot', { forceRemoteUpdates })
export const ensureManagedFiles = (): Promise<EnsureManagedFilesResult> => invoke('ensure_managed_files')
export const restoreHashesFromDisk = (): Promise<void> => invoke('restore_hashes_from_disk')
export const downloadBinaries = async (forceAll = false): Promise<void> => invoke('download_binaries', { forceAll })
export const applyCoreFileUpdates = async (): Promise<void> => invoke('apply_core_file_updates')
export const refreshListsIfStale = (): Promise<number> => invoke('refresh_lists_if_stale')
export const restoreDefaultFilters = (): Promise<void> => invoke('restore_default_filters')
export const getWinwsPath = (): Promise<string> => invoke('get_winws_path')
export const startWinws = (args: string[], tcpPorts: string, udpPorts: string): Promise<number> => invoke('start_winws', { args, tcpPorts, udpPorts })
export const stopWinws = (): Promise<void> => invoke('stop_winws')
export const isWinwsRunning = (): Promise<boolean> => invoke('is_winws_running')
export const killWindivertService = (): Promise<void> => invoke('kill_windivert_service')
export const getRunningPid = (): Promise<number> => invoke('get_running_pid')
export const checkAndRecoverOrphan = (): Promise<number | null> => invoke('check_and_recover_orphan')
export const openAppDirectory = (): Promise<void> => invoke('open_app_directory')
export const openFiltersDirectory = (): Promise<void> => invoke('open_filters_directory')
export const getFiltersPath = (): Promise<string> => invoke('get_filters_path')
export const getReservedFilterFilenames = (): Promise<string[]> => invoke('get_reserved_filter_filenames')
export const isAutostartEnabled = (): Promise<boolean> => invoke('is_autostart_enabled')
export const setAutostartEnabled = (enabled: boolean): Promise<void> => invoke('set_autostart_enabled', { enabled })
export const wasLaunchedFromAutostart = (): Promise<boolean> => invoke('was_launched_from_autostart')
export const getAppVersion = (): Promise<string> => getVersion()
export const setWindowMaterial = (material: WindowMaterial): Promise<void> => invoke('set_window_material', { material })

export function saveFilterFile(filename: string, content: string): Promise<void> {
  return invoke('save_filter_file', { filename, content })
}

export function loadFilterFile(filename: string): Promise<string> {
  return invoke('load_filter_file', { filename })
}

export function deleteFilterFile(filename: string): Promise<void> {
  return invoke('delete_filter_file', { filename })
}

export function resolvePlaceholders(content: string, placeholders: { name: string, path: string }[]): Promise<string> {
  return invoke('resolve_placeholders', { content, placeholders })
}

export const checkTcpTimestamps = (): Promise<boolean> => invoke('check_tcp_timestamps')
export const enableTcpTimestamps = (): Promise<void> => invoke('enable_tcp_timestamps')
export const setConnectedState = (connected: boolean): Promise<void> => invoke('set_connected_state', { connected })
export const updateListMode = (mode: ListMode): Promise<void> => invoke('update_list_mode', { mode })

function createAsyncListener<T>(eventName: string, callback: (payload: T) => void): (() => void) {
  let unlisten: (() => void) | null = null
  let called = false
  let registrationFailed = false
  let registrationError: unknown = null
  const listenPromise = listen<T>(eventName, event => callback(event.payload))
  listenPromise
    .then((fn) => {
      if (!called)
        unlisten = fn
      else
        fn()
    })
    .catch((e) => {
      console.error(`Failed to register ${eventName} listener:`, e)
      registrationFailed = true
      registrationError = e
    })
  return () => {
    called = true
    if (unlisten)
      unlisten()
    else if (registrationFailed)
      console.error(`${eventName} listener cleanup called but registration had failed:`, registrationError)
  }
}

export function onTrayConnectToggle(callback: () => void): (() => void) {
  return createAsyncListener('tray-connect-toggle', callback)
}

export function onListModeChanged(callback: (mode: ListMode) => void): (() => void) {
  return createAsyncListener<ListMode>('list-mode-changed', callback)
}

export function onFilesHealthChanged(callback: (payload: FileHealthChangedPayload) => void): (() => void) {
  return createAsyncListener<FileHealthChangedPayload>('files-health-changed', callback)
}

export function onFilesHealthWatchError(callback: (message: string) => void): (() => void) {
  return createAsyncListener<string>('files-health-watch-error', callback)
}
