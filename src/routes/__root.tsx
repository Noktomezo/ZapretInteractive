import { createRootRoute, Outlet } from '@tanstack/react-router'
import { OverlayScrollbarsComponent } from 'overlayscrollbars-react'
import { Toaster } from 'sonner'
import { Sidebar } from '@/components/features/Sidebar'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { TooltipProvider } from '@/components/ui/tooltip'
import { useSidebarStore } from '@/stores/sidebar.store'
import { TitleBar } from '../components/features/TitleBar'

function RootLayout() {
  const { collapsed, setCollapsed } = useSidebarStore()

  return (
    <TooltipProvider>
      <div className="min-h-full bg-background">
        <TitleBar />
        <div className="h-screen overflow-hidden pt-8">
          <SidebarProvider open={!collapsed} onOpenChange={open => setCollapsed(!open)}>
            <Sidebar />
            <SidebarInset className="h-full min-h-0">
              <OverlayScrollbarsComponent
                defer
                options={{ scrollbars: { theme: 'os-theme-custom', autoHide: 'leave' } }}
                className="h-full flex-1"
              >
                <Outlet />
              </OverlayScrollbarsComponent>
            </SidebarInset>
          </SidebarProvider>
        </div>
      </div>
      <Toaster position="bottom-right" />
    </TooltipProvider>
  )
}

export const Route = createRootRoute({
  component: RootLayout,
})

