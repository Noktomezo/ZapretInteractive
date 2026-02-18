import { Monitor, Moon, Sun } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useThemeStore } from '@/stores/theme.store'

const themes = [
  { value: 'light' as const, label: 'Светлая', icon: Sun },
  { value: 'dark' as const, label: 'Тёмная', icon: Moon },
  { value: 'system' as const, label: 'Системная', icon: Monitor },
]

export function ThemeSwitcher() {
  const { theme, setTheme } = useThemeStore()

  return (
    <div className="flex gap-2">
      {themes.map(({ value, label, icon: Icon }) => (
        <Button
          key={value}
          variant={theme === value ? 'default' : 'outline'}
          size="sm"
          onClick={() => setTheme(value)}
          className={cn('flex-1 cursor-pointer', theme === value && 'pointer-events-none')}
        >
          <Icon className="w-4 h-4 mr-2" />
          {label}
        </Button>
      ))}
    </div>
  )
}
