import { createRootRoute, Outlet, useLocation } from '@tanstack/react-router'
import { useEffect } from 'react'
import { Toaster } from 'sonner'
import { Sidebar } from '@/components/features/Sidebar'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { TooltipProvider } from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'
import { useConfigStore } from '@/stores/config.store'
import { useSidebarStore } from '@/stores/sidebar.store'
import { useThemeStore } from '@/stores/theme.store'
import { TitleBar } from '../components/features/TitleBar'

function RootLayout() {
  const { collapsed, setCollapsed } = useSidebarStore()
  const location = useLocation()
  const config = useConfigStore(state => state.config)
  const loadConfig = useConfigStore(state => state.load)
  const resolvedTheme = useThemeStore(state => state.resolvedTheme)
  const windowMaterial = config?.windowMaterial ?? 'acrylic'
  const materialEnabled = windowMaterial !== 'none'

  useEffect(() => {
    void loadConfig()
  }, [loadConfig])

  return (
    <TooltipProvider>
      <div className={cn('min-h-full', materialEnabled ? 'app-glass-surface bg-transparent' : 'app-opaque-surface bg-background')}>
        <SidebarProvider className="flex-col" open={!collapsed} onOpenChange={open => setCollapsed(!open)}>
          <TitleBar />
          <div className="flex h-screen overflow-hidden pt-11">
            <Sidebar />
            <SidebarInset className="h-full min-h-0 overflow-hidden rounded-tl-xl">
              <div className="h-full min-h-0 flex-1 overflow-hidden">
                <div
                  key={location.pathname}
                  className={cn(
                    'flex h-full min-h-0 flex-col',
                    location.pathname !== '/' && 'page-fade-in',
                  )}
                >
                  <Outlet />
                </div>
              </div>
            </SidebarInset>
          </div>
        </SidebarProvider>
      </div>
      <Toaster position="bottom-right" theme={resolvedTheme} />
    </TooltipProvider>
  )
}

export const Route = createRootRoute({
  component: RootLayout,
})
