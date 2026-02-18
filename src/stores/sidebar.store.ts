import { create } from 'zustand'

interface SidebarStore {
  collapsed: boolean
  toggle: () => void
  setCollapsed: (collapsed: boolean) => void
}

export const useSidebarStore = create<SidebarStore>((set, get) => ({
  collapsed: (() => {
    if (typeof window === 'undefined')
      return false
    const stored = localStorage.getItem('sidebar-collapsed')
    return stored === 'true'
  })(),

  toggle: () => {
    const newState = !get().collapsed
    localStorage.setItem('sidebar-collapsed', String(newState))
    set({ collapsed: newState })
  },

  setCollapsed: (collapsed) => {
    localStorage.setItem('sidebar-collapsed', String(collapsed))
    set({ collapsed })
  },
}))
