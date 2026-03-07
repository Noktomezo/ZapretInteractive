import { Link, useLocation } from '@tanstack/react-router'
import type { LucideIcon } from 'lucide-react'
import {
  FileCode,
  Filter,
  Home,
  Layers,
  Settings,
} from 'lucide-react'
import {
  Sidebar as AppSidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
  SidebarTrigger,
  useSidebar,
} from '@/components/ui/sidebar'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'

const mainNavItems = [
  { path: '/', label: 'Главная', icon: Home },
  { path: '/strategies', label: 'Стратегии', icon: Layers },
  { path: '/filters', label: 'Фильтры', icon: Filter },
  { path: '/placeholders', label: 'Плейсхолдеры', icon: FileCode },
]

const footerNavItem = { path: '/settings', label: 'Настройки', icon: Settings }

function SidebarNavItem({
  path,
  label,
  icon: Icon,
}: {
  path: string
  label: string
  icon: LucideIcon
}) {
  const location = useLocation()
  const currentPath = location.pathname
  const { open } = useSidebar()
  const isActive = currentPath === path

  const button = (
    <SidebarMenuButton asChild isActive={isActive}>
      <Link to={path} className="flex w-full items-center overflow-hidden">
        <span className="flex size-7 shrink-0 items-center justify-center">
          <Icon className="size-4 shrink-0" />
        </span>
        <span className="min-w-0 flex-1 overflow-hidden">
          <span
            className={[
              'block whitespace-nowrap text-left transition-[transform,opacity] duration-200 ease-out will-change-transform',
              open ? 'translate-x-0 opacity-100' : '-translate-x-full opacity-0',
            ].join(' ')}
            aria-hidden={!open}
          >
            {label}
          </span>
        </span>
      </Link>
    </SidebarMenuButton>
  )

  return (
    <SidebarMenuItem>
      {open
        ? button
        : (
            <Tooltip>
              <TooltipTrigger asChild>{button}</TooltipTrigger>
              <TooltipContent side="right">{label}</TooltipContent>
            </Tooltip>
          )}
    </SidebarMenuItem>
  )
}

function SidebarNav() {
  return (
    <SidebarGroup>
      <SidebarGroupContent>
        <SidebarMenu>
          {mainNavItems.map(item => (
            <SidebarNavItem key={item.path} {...item} />
          ))}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  )
}

export function Sidebar() {
  return (
    <AppSidebar className="h-full border-r bg-card">
      <SidebarHeader className="justify-end">
        <SidebarTrigger />
      </SidebarHeader>
      <SidebarContent>
        <SidebarNav />
      </SidebarContent>
      <SidebarFooter>
        <SidebarMenu>
          <SidebarNavItem {...footerNavItem} />
        </SidebarMenu>
      </SidebarFooter>
      <SidebarRail />
    </AppSidebar>
  )
}

