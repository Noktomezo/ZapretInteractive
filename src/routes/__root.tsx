import { createRootRoute, Outlet } from '@tanstack/react-router'
import { OverlayScrollbarsComponent } from 'overlayscrollbars-react'
import { TooltipProvider } from '@/components/ui/tooltip'
import { Sidebar } from '../components/features/Sidebar'
import { TitleBar } from '../components/features/TitleBar'

export const Route = createRootRoute({
  component: () => (
    <TooltipProvider>
      <TitleBar />
      <div className="flex h-screen bg-background pt-8">
        <Sidebar />
        <OverlayScrollbarsComponent
          defer
          options={{ scrollbars: { theme: 'os-theme-custom', autoHide: 'leave' } }}
          className="flex-1"
        >
          <Outlet />
        </OverlayScrollbarsComponent>
      </div>
    </TooltipProvider>
  ),
})
