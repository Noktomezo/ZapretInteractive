import { createRootRoute, Outlet, useLocation } from '@tanstack/react-router'
import { Toaster } from 'sonner'
import { Sidebar } from '@/components/features/Sidebar'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { TooltipProvider } from '@/components/ui/tooltip'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { cn } from '@/lib/utils'
import { useConfigStore } from '@/stores/config.store'
import { useSidebarStore } from '@/stores/sidebar.store'
import { useThemeStore } from '@/stores/theme.store'
import { TitleBar } from '../components/features/TitleBar'
import 'sonner/dist/styles.css'

function RootLayout() {
  const { collapsed, setCollapsed } = useSidebarStore()
  const location = useLocation()
  const config = useConfigStore(state => state.config)
  const loadConfig = useConfigStore(state => state.load)
  const resolvedTheme = useThemeStore(state => state.resolvedTheme)
  const windowMaterial = config?.windowMaterial ?? 'acrylic'
  const materialEnabled = windowMaterial !== 'none'

  useMountEffect(() => {
    void loadConfig()
  })

  return (
    <TooltipProvider>
      <>
        <div
          data-theme={resolvedTheme}
          data-webview-material={windowMaterial}
          className={cn('min-h-full', materialEnabled ? 'app-glass-surface bg-transparent' : 'app-opaque-surface bg-background')}
        >
          <SidebarProvider className="flex-col" open={!collapsed} onOpenChange={open => setCollapsed(!open)}>
            <TitleBar />
            <div className="flex h-screen overflow-hidden pt-[30px]">
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
        <div data-theme={resolvedTheme}>
          <Toaster
            position="bottom-right"
            theme={resolvedTheme}
            toastOptions={{
              unstyled: true,
              classNames: {
                toast: 'group pointer-events-auto relative flex w-[min(22.5rem,calc(100vw-2rem))] cursor-grab flex-wrap items-start gap-x-3 gap-y-2 rounded-lg border border-border bg-popover p-4 pr-11 text-popover-foreground shadow-[0_16px_34px_color-mix(in_oklab,var(--foreground)_12%,transparent),0_6px_16px_color-mix(in_oklab,var(--foreground)_8%,transparent)] backdrop-blur-xl active:cursor-grabbing',
                content: 'min-w-0 flex-1 self-center',
                title: 'text-sm leading-5 font-medium text-popover-foreground',
                description: 'mt-0.5 text-xs leading-5 text-muted-foreground',
                closeButton: 'absolute top-3 right-3 inline-flex size-7 items-center justify-center rounded-md border border-border bg-background/88 text-foreground/82 transition-colors hover:bg-accent hover:text-accent-foreground',
                cancelButton: 'mt-1 inline-flex h-8 items-center justify-center rounded-md border border-border bg-background/92 px-3 text-xs font-medium text-foreground transition-colors hover:bg-accent',
                actionButton: 'mt-1 inline-flex h-8 items-center justify-center rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/92',
                success: 'border-border bg-popover text-popover-foreground [&_[data-icon]]:text-success',
                warning: 'border-border bg-popover text-popover-foreground [&_[data-icon]]:text-warning',
                error: 'border-border bg-popover text-popover-foreground [&_[data-icon]]:text-destructive',
                info: 'border-border bg-popover text-popover-foreground [&_[data-icon]]:text-primary',
                loading: 'border-border bg-popover text-popover-foreground',
                default: 'border-border bg-popover text-popover-foreground',
                icon: 'flex size-5 shrink-0 self-center items-center justify-center text-current',
              },
            }}
          />
        </div>
      </>
    </TooltipProvider>
  )
}

export const Route = createRootRoute({
  component: RootLayout,
})
