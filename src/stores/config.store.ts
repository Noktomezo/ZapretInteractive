import { create } from 'zustand'
import type { AppConfig, Category, Strategy, Placeholder, GlobalPorts, Filter, ListMode } from '../lib/types'
import * as tauri from '../lib/tauri'

interface ConfigStore {
  config: AppConfig | null
  loading: boolean
  error: string | null
  load: () => Promise<void>
  save: () => Promise<void>
  reset: () => Promise<void>

  setGlobalPorts: (ports: GlobalPorts) => void
  setFilters: (filters: Filter[]) => void
  setListMode: (mode: ListMode) => void
  setMinimizeToTray: (enabled: boolean) => void
  addCategory: (name: string) => void
  updateCategory: (id: string, name: string) => void
  deleteCategory: (id: string) => void
  reorderCategories: (oldIndex: number, newIndex: number) => void
  addStrategy: (categoryId: string, name: string, content: string) => void
  updateStrategy: (categoryId: string, strategyId: string, updates: Partial<Strategy>) => void
  deleteStrategy: (categoryId: string, strategyId: string) => void
  setActiveStrategy: (categoryId: string, strategyId: string) => void
  clearActiveStrategy: (categoryId: string, strategyId: string) => void
  clearAllActiveStrategies: (categoryId: string) => void
  addPlaceholder: (name: string, path: string) => void
  updatePlaceholder: (index: number, name: string, path: string) => void
  deletePlaceholder: (index: number) => void
}

export const useConfigStore = create<ConfigStore>((set, get) => ({
  config: null,
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null })
    try {
      const config = await tauri.loadConfig()
      set({ config, loading: false })
    } catch (e) {
      set({ error: String(e), loading: false })
    }
  },

  save: async () => {
    const { config } = get()
    if (config) {
      await tauri.saveConfig(config)
    }
  },

  reset: async () => {
    set({ loading: true })
    try {
      const config = await tauri.resetConfig()
      set({ config, loading: false })
    } catch (e) {
      set({ error: String(e), loading: false })
    }
  },

  setGlobalPorts: (ports) => {
    const { config } = get()
    if (config) {
      set({ config: { ...config, global_ports: ports } })
    }
  },

setFilters: (filters) => {
    const { config } = get()
    if (config) {
      set({ config: { ...config, filters } })
    }
  },

  setListMode: (mode) => {
    const { config } = get()
    if (config) {
      set({ config: { ...config, listMode: mode } })
    }
  },

  setMinimizeToTray: (enabled) => {
    const { config } = get()
    if (config) {
      set({ config: { ...config, minimizeToTray: enabled } })
    }
  },

  addCategory: (name) => {
    const { config } = get()
    if (config) {
      const newCategory: Category = {
        id: crypto.randomUUID(),
        name,
        strategies: [],
      }
      set({ config: { ...config, categories: [...config.categories, newCategory] } })
    }
  },

  updateCategory: (id, name) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.map((c) =>
        c.id === id ? { ...c, name } : c
      )
      set({ config: { ...config, categories } })
    }
  },

  deleteCategory: (id) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.filter((c) => c.id !== id)
      set({ config: { ...config, categories } })
    }
  },

  reorderCategories: (oldIndex, newIndex) => {
    const { config } = get()
    if (config && oldIndex !== newIndex) {
      const categories = [...config.categories]
      const [removed] = categories.splice(oldIndex, 1)
      categories.splice(newIndex, 0, removed)
      set({ config: { ...config, categories } })
    }
  },

  addStrategy: (categoryId, name, content) => {
    const { config } = get()
    if (config) {
      const newStrategy: Strategy = {
        id: crypto.randomUUID(),
        name,
        content,
        active: false,
      }
      const categories = config.categories.map((c) =>
        c.id === categoryId
          ? { ...c, strategies: [...c.strategies, newStrategy] }
          : c
      )
      set({ config: { ...config, categories } })
    }
  },

  updateStrategy: (categoryId, strategyId, updates) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.map((c) =>
        c.id === categoryId
          ? {
              ...c,
              strategies: c.strategies.map((s) =>
                s.id === strategyId ? { ...s, ...updates } : s
              ),
            }
          : c
      )
      set({ config: { ...config, categories } })
    }
  },

  deleteStrategy: (categoryId, strategyId) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.map((c) =>
        c.id === categoryId
          ? { ...c, strategies: c.strategies.filter((s) => s.id !== strategyId) }
          : c
      )
      set({ config: { ...config, categories } })
    }
  },

  setActiveStrategy: (categoryId, strategyId) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.map((c) =>
        c.id === categoryId
          ? {
              ...c,
              strategies: c.strategies.map((s) => ({
                ...s,
                active: s.id === strategyId,
              })),
            }
          : c
      )
      set({ config: { ...config, categories } })
    }
  },

  clearActiveStrategy: (categoryId, strategyId) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.map((c) =>
        c.id === categoryId
          ? {
              ...c,
              strategies: c.strategies.map((s) =>
                s.id === strategyId ? { ...s, active: false } : s
              ),
            }
          : c
      )
      set({ config: { ...config, categories } })
    }
  },

  clearAllActiveStrategies: (categoryId) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.map((c) =>
        c.id === categoryId
          ? {
              ...c,
              strategies: c.strategies.map((s) => ({ ...s, active: false })),
            }
          : c
      )
      set({ config: { ...config, categories } })
    }
  },

  addPlaceholder: (name, path) => {
    const { config } = get()
    if (config) {
      const newPlaceholder: Placeholder = { name, path }
      set({
        config: { ...config, placeholders: [...config.placeholders, newPlaceholder] },
      })
    }
  },

  updatePlaceholder: (index, name, path) => {
    const { config } = get()
    if (config) {
      const placeholders = [...config.placeholders]
      placeholders[index] = { name, path }
      set({ config: { ...config, placeholders } })
    }
  },

  deletePlaceholder: (index) => {
    const { config } = get()
    if (config) {
      const placeholders = config.placeholders.filter((_, i) => i !== index)
      set({ config: { ...config, placeholders } })
    }
  },
}))