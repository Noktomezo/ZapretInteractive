import { create } from 'zustand'

type Theme = 'light' | 'dark' | 'system'

interface ThemeStore {
  theme: Theme
  resolvedTheme: 'light' | 'dark'
  setTheme: (theme: Theme) => void
  initTheme: () => void
}

function resolveTheme(theme: Theme): 'light' | 'dark' {
  if (theme === 'system') {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }
  return theme
}

function applyTheme(theme: 'light' | 'dark') {
  document.documentElement.classList.remove('light', 'dark')
  document.documentElement.classList.add(theme)
}

export const useThemeStore = create<ThemeStore>((set, get) => ({
  theme: 'system',
  resolvedTheme: 'light',

  initTheme: () => {
    const stored = localStorage.getItem('theme') as Theme | null
    const theme = stored || 'system'
    const resolved = resolveTheme(theme)
    applyTheme(resolved)
    set({ theme, resolvedTheme: resolved })

    const media = window.matchMedia('(prefers-color-scheme: dark)')
    media.addEventListener('change', (e) => {
      if (get().theme === 'system') {
        const resolved = e.matches ? 'dark' : 'light'
        applyTheme(resolved)
        set({ resolvedTheme: resolved })
      }
    })
  },

  setTheme: (theme) => {
    const resolved = resolveTheme(theme)
    applyTheme(resolved)
    localStorage.setItem('theme', theme)
    set({ theme, resolvedTheme: resolved })
  },
}))
