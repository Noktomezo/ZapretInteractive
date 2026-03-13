import { createRootRoute, Outlet } from '@tanstack/react-router'
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
      <div className="app-glass-surface min-h-full bg-transparent">
        <SidebarProvider className="flex-col" open={!collapsed} onOpenChange={open => setCollapsed(!open)}>
          <TitleBar />
          <div className="flex h-screen overflow-hidden pt-10">
            <Sidebar />
            <SidebarInset className="h-full min-h-0 overflow-hidden rounded-tl-lg">
              <div className="h-full min-h-0 flex-1 overflow-hidden">
                <div className="flex h-full min-h-0 flex-col animate-in fade-in duration-200">
                  <Outlet />
                </div>
              </div>
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
