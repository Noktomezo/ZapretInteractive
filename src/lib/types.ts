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
}
