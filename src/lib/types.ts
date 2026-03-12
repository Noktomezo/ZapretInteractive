export interface GlobalPorts {
  tcp: string
  udp: string
}

export interface DownloadProgress {
  current: number
  total: number
  filename: string
  phase: 'binaries' | 'fake' | 'lists' | 'filters'
}

export interface AppHealthSnapshot {
  binaries_ok: boolean
  missing_critical_files: string[]
  available_updates: string[]
  available_updates_checked: boolean
  config_missing: boolean
  lists_last_updated_at?: number | null
}

export interface Strategy {
  id: string
  name: string
  content: string
  active: boolean
}

export interface Category {
  id: string
  name: string
  strategies: Strategy[]
}

export interface Placeholder {
  name: string
  path: string
}

export interface Filter {
  id: string
  name: string
  filename: string
  active: boolean
  content: string
}

export type ListMode = 'exclude' | 'ipset'

export interface AppConfig {
  global_ports: GlobalPorts
  categories: Category[]
  placeholders: Placeholder[]
  filters: Filter[]
  binaries_path: string
  listMode?: ListMode
  minimizeToTray?: boolean
  launchToTray?: boolean
  connectOnAutostart?: boolean
  coreFileUpdatePromptsEnabled?: boolean
}

export interface EnsureManagedFilesResult {
  restored_files: string[]
  config_restored: boolean
  config_reloaded: boolean
  unrecoverable_filters: string[]
}
