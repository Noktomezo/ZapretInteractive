import { createRootRoute, Outlet, useRouterState } from '@tanstack/react-router'
import { OverlayScrollbarsComponent } from 'overlayscrollbars-react'
import { Toaster } from 'sonner'
import { Sidebar } from '@/components/features/Sidebar'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { TooltipProvider } from '@/components/ui/tooltip'
import { useSidebarStore } from '@/stores/sidebar.store'
import { TitleBar } from '../components/features/TitleBar'

function RootLayout() {
  const { collapsed, setCollapsed } = useSidebarStore()
  const pathname = useRouterState({ select: state => state.location.pathname })

  return (
    <TooltipProvider>
      <div className="app-glass-surface min-h-full bg-transparent">
        <SidebarProvider className="flex-col" open={!collapsed} onOpenChange={open => setCollapsed(!open)}>
          <TitleBar />
          <div className="flex h-screen overflow-hidden pt-10">
            <Sidebar />
            <SidebarInset className="h-full min-h-0 overflow-hidden rounded-tl-lg">
              <OverlayScrollbarsComponent
                defer
                options={{ scrollbars: { theme: 'os-theme-custom', autoHide: 'leave' } }}
                className="h-full flex-1"
              >
                <div key={pathname} className="h-full min-h-full">
                  <Outlet />
                </div>
              </OverlayScrollbarsComponent>
            </SidebarInset>
          </div>
        </SidebarProvider>
      </div>
      <Toaster position="bottom-right" />
    </TooltipProvider>
  )
}

export const Route = createRootRoute({
  component: RootLayout,
})
