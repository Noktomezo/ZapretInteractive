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
}

export interface DnsProxyStatus {
  installed: boolean
  running: boolean
  appManaged: boolean
  moduleAvailable: boolean
  configPath: string
  serviceName: string
}

export interface DnsLatencyResult {
  url: string
  reachable: boolean
  latencyMs: number | null
  error?: string | null
}

export interface TgWsProxyStatus {
  running: boolean
  moduleAvailable: boolean
  binaryPath: string
  logPath: string
  pid?: number | null
}

export interface Strategy {
  id: string
  name: string
  content: string
  active: boolean
  system?: boolean
  systemBaseName?: string
  systemBaseContent?: string
}

export interface Category {
  id: string
  name: string
  strategies: Strategy[]
  system?: boolean
  systemBaseName?: string
}

export interface Placeholder {
  name: string
  path: string
  system?: boolean
  systemBaseName?: string
  systemBasePath?: string
}

export interface Filter {
  id: string
  name: string
  filename: string
  active: boolean
  content: string
  system?: boolean
  systemBaseName?: string
  systemBaseFilename?: string
  systemBaseContent?: string
  systemBaseActive?: boolean
}

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'disconnecting' | 'error'
export type DiscordPresenceActivityType = 'playing' | 'listening' | 'watching' | 'competing'
export type ListMode = 'exclude' | 'ipset'
export type WindowMaterial = 'none' | 'acrylic' | 'mica' | 'tabbed'

export interface AppConfig {
  global_ports: GlobalPorts
  categories: Category[]
  placeholders: Placeholder[]
  filters: Filter[]
  binaries_path: string
  listMode?: ListMode
  dnsPresetId?: string
  dnsBootstrapResolvers?: string[]
  dnsAcceleratorEnabled?: boolean
  dnsModuleEnabled?: boolean
  tgWsProxyPort?: number
  tgWsProxySecret?: string
  tgWsProxyModuleEnabled?: boolean
  discordPresenceEnabled?: boolean
  discordPresenceActivityType?: DiscordPresenceActivityType
  minimizeToTray?: boolean
  launchToTray?: boolean
  connectOnAutostart?: boolean
  coreFileUpdatePromptsEnabled?: boolean
  appAutoUpdatesEnabled?: boolean
  windowMaterial?: WindowMaterial
  systemRemovedCategoryIds?: string[]
  systemRemovedStrategyKeys?: string[]
  systemRemovedPlaceholderNames?: string[]
  systemRemovedFilterIds?: string[]
  systemSyncInitialized?: boolean
}

export interface EnsureManagedFilesResult {
  restored_files: string[]
  config_restored: boolean
  config_reloaded: boolean
  unrecoverable_filters: string[]
}

export interface WindowMaterialCapabilities {
  acrylic: boolean
  mica: boolean
  tabbed: boolean
}
