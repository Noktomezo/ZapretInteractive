import type { AppConfig, Category, Filter, GlobalPorts, ListMode, Placeholder, Strategy } from '../lib/types'
import { toast } from 'sonner'
import { create } from 'zustand'
import * as tauri from '../lib/tauri'
import { useConnectionStore } from './connection.store'

let saveTimeoutId: number | null = null
let savePromise: Promise<void> | null = null
let queuedSaveAfterCurrent = false
let lastAutosaveErrorKey: string | null = null

function cloneConfig(config: AppConfig): AppConfig {
  return structuredClone(config)
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

function reportAutosaveError(reason: string, error: unknown) {
  const message = getErrorMessage(error)
  const dedupeKey = `${reason}:${message}`
  if (lastAutosaveErrorKey === dedupeKey) {
    return
  }

  lastAutosaveErrorKey = dedupeKey
  useConnectionStore.getState().addLog(`Автосохранение настроек завершилось ошибкой (${reason}): ${message}`)
  toast.error('Изменения не удалось сохранить автоматически')
}

interface ConfigStore {
  config: AppConfig | null
  loading: boolean
  error: string | null
  dirty: boolean
  isSaving: boolean
  load: () => Promise<void>
  reload: () => Promise<void>
  save: () => Promise<void>
  saveNow: () => Promise<void>
  scheduleSave: (reason: string, debounceMs?: number) => void
  revertTo: (config: AppConfig) => void
  reset: () => Promise<void>

  setGlobalPorts: (ports: GlobalPorts) => void
  setFilters: (filters: Filter[]) => void
  setListMode: (mode: ListMode) => void
  applyPersistedListMode: (mode: ListMode) => void
  setMinimizeToTray: (enabled: boolean) => void
  setLaunchToTray: (enabled: boolean) => void
  setConnectOnAutostart: (enabled: boolean) => void
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
  setPlaceholders: (placeholders: Placeholder[]) => void
}

export const useConfigStore = create<ConfigStore>((set, get) => ({
  config: null,
  loading: false,
  error: null,
  dirty: false,
  isSaving: false,

  load: async () => {
    if (get().config) {
      return
    }

    set({ loading: true, error: null })
    try {
      const config = await tauri.loadConfig()
      lastAutosaveErrorKey = null
      set({ config, loading: false, dirty: false, isSaving: false })
    }
    catch (e) {
      set({ error: String(e), loading: false })
    }
  },

  reload: async () => {
    set({ error: null })
    try {
      const config = await tauri.loadConfig()
      lastAutosaveErrorKey = null
      set({ config, loading: false, dirty: false, isSaving: false, error: null })
    }
    catch (e) {
      set({ error: String(e), loading: false, isSaving: false })
      throw e
    }
  },

  save: async () => {
    await get().saveNow()
  },

  saveNow: async () => {
    if (saveTimeoutId !== null) {
      window.clearTimeout(saveTimeoutId)
      saveTimeoutId = null
    }

    if (savePromise) {
      queuedSaveAfterCurrent = true
      await savePromise
      if (get().dirty) {
        return get().saveNow()
      }
      return
    }

    const { config } = get()
    if (!config) {
      return
    }

    const currentConfig = config
    const snapshot = cloneConfig(currentConfig)

    savePromise = (async () => {
      set({ isSaving: true, error: null })
      try {
        await tauri.saveConfig(snapshot)
        lastAutosaveErrorKey = null
        set(state => ({
          error: null,
          isSaving: false,
          dirty: state.config === currentConfig ? false : state.dirty,
        }))
      }
      catch (e) {
        set({ error: String(e), isSaving: false })
        throw e
      }
      finally {
        savePromise = null
      }
    })()

    try {
      await savePromise
    }
    finally {
      if (queuedSaveAfterCurrent) {
        queuedSaveAfterCurrent = false
        if (get().dirty) {
          await get().saveNow()
        }
      }
    }
  },

  scheduleSave: (reason, debounceMs = 400) => {
    if (!get().config) {
      return
    }

    if (saveTimeoutId !== null) {
      window.clearTimeout(saveTimeoutId)
    }

    set({ dirty: true })
    saveTimeoutId = window.setTimeout(() => {
      saveTimeoutId = null
      void get().saveNow().catch((error) => {
        reportAutosaveError(reason, error)
      })
    }, debounceMs)
  },

  revertTo: (config) => {
    set({ config: cloneConfig(config), dirty: false, error: null })
  },

  reset: async () => {
    set({ loading: true })
    try {
      const config = await tauri.resetConfig()
      lastAutosaveErrorKey = null
      set({ config, loading: false, dirty: false, isSaving: false })
    }
    catch (e) {
      set({ error: String(e), loading: false })
    }
  },

  setGlobalPorts: (ports) => {
    const { config } = get()
    if (config)
      set({ config: { ...config, global_ports: ports }, dirty: true })
  },

  setFilters: (filters) => {
    const { config } = get()
    if (config)
      set({ config: { ...config, filters }, dirty: true })
  },

  setListMode: (mode) => {
    const { config } = get()
    if (config)
      set({ config: { ...config, listMode: mode }, dirty: true })
  },

  applyPersistedListMode: (mode) => {
    const { config } = get()
    if (config)
      set({ config: { ...config, listMode: mode } })
  },

  setMinimizeToTray: (enabled) => {
    const { config } = get()
    if (config)
      set({ config: { ...config, minimizeToTray: enabled }, dirty: true })
  },

  setLaunchToTray: (enabled) => {
    const { config } = get()
    if (config)
      set({ config: { ...config, launchToTray: enabled }, dirty: true })
  },

  setConnectOnAutostart: (enabled) => {
    const { config } = get()
    if (config)
      set({ config: { ...config, connectOnAutostart: enabled }, dirty: true })
  },

  addCategory: (name) => {
    const { config } = get()
    if (config) {
      const newCategory: Category = {
        id: crypto.randomUUID(),
        name,
        strategies: [],
      }
      set({ config: { ...config, categories: [...config.categories, newCategory] }, dirty: true })
    }
  },

  updateCategory: (id, name) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.map(c =>
        c.id === id ? { ...c, name } : c,
      )
      set({ config: { ...config, categories }, dirty: true })
    }
  },

  deleteCategory: (id) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.filter(c => c.id !== id)
      set({ config: { ...config, categories }, dirty: true })
    }
  },

  reorderCategories: (oldIndex, newIndex) => {
    const { config } = get()
    if (config && oldIndex !== newIndex) {
      const categories = [...config.categories]
      const [removed] = categories.splice(oldIndex, 1)
      categories.splice(newIndex, 0, removed)
      set({ config: { ...config, categories }, dirty: true })
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
      const categories = config.categories.map(c =>
        c.id === categoryId
          ? { ...c, strategies: [...c.strategies, newStrategy] }
          : c,
      )
      set({ config: { ...config, categories }, dirty: true })
    }
  },

  updateStrategy: (categoryId, strategyId, updates) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.map(c =>
        c.id === categoryId
          ? {
              ...c,
              strategies: c.strategies.map(s =>
                s.id === strategyId ? { ...s, ...updates } : s,
              ),
            }
          : c,
      )
      set({ config: { ...config, categories }, dirty: true })
    }
  },

  deleteStrategy: (categoryId, strategyId) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.map(c =>
        c.id === categoryId
          ? { ...c, strategies: c.strategies.filter(s => s.id !== strategyId) }
          : c,
      )
      set({ config: { ...config, categories }, dirty: true })
    }
  },

  setActiveStrategy: (categoryId, strategyId) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.map(c =>
        c.id === categoryId
          ? {
              ...c,
              strategies: c.strategies.map(s => ({
                ...s,
                active: s.id === strategyId,
              })),
            }
          : c,
      )
      set({ config: { ...config, categories }, dirty: true })
    }
  },

  clearActiveStrategy: (categoryId, strategyId) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.map(c =>
        c.id === categoryId
          ? {
              ...c,
              strategies: c.strategies.map(s =>
                s.id === strategyId ? { ...s, active: false } : s,
              ),
            }
          : c,
      )
      set({ config: { ...config, categories }, dirty: true })
    }
  },

  clearAllActiveStrategies: (categoryId) => {
    const { config } = get()
    if (config) {
      const categories = config.categories.map(c =>
        c.id === categoryId
          ? {
              ...c,
              strategies: c.strategies.map(s => ({ ...s, active: false })),
            }
          : c,
      )
      set({ config: { ...config, categories }, dirty: true })
    }
  },

  addPlaceholder: (name, path) => {
    const { config } = get()
    if (config) {
      const newPlaceholder: Placeholder = { name, path }
      set({
        config: { ...config, placeholders: [...config.placeholders, newPlaceholder] },
        dirty: true,
      })
    }
  },

  updatePlaceholder: (index, name, path) => {
    const { config } = get()
    if (config) {
      const placeholders = [...config.placeholders]
      placeholders[index] = { name, path }
      set({ config: { ...config, placeholders }, dirty: true })
    }
  },

  deletePlaceholder: (index) => {
    const { config } = get()
    if (config) {
      const placeholders = config.placeholders.filter((_, i) => i !== index)
      set({ config: { ...config, placeholders }, dirty: true })
    }
  },

  setPlaceholders: (placeholders) => {
    const { config } = get()
    if (config) {
      set({ config: { ...config, placeholders }, dirty: true })
    }
  },
}))
