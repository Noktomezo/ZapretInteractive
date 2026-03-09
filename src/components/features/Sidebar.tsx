import type { LucideIcon } from 'lucide-react'
import { Link, useLocation } from '@tanstack/react-router'
import {
  FileCode,
  Filter,
  Home,
  Layers,
  Logs,
  Settings,
} from 'lucide-react'
import {
  Sidebar as AppSidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
  useSidebar,
} from '@/components/ui/sidebar'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'
import { useAppStore } from '@/stores/app.store'

const mainNavItems = [
  { path: '/', label: 'Главная', icon: Home },
  { path: '/strategies', label: 'Стратегии', icon: Layers, requiresFiles: true },
  { path: '/filters', label: 'Фильтры', icon: Filter, requiresFiles: true },
  { path: '/placeholders', label: 'Плейсхолдеры', icon: FileCode, requiresFiles: true },
  { path: '/logs', label: 'Логи', icon: Logs },
]

const footerNavItem = { path: '/settings', label: 'Настройки', icon: Settings }

function SidebarNavItem({
  path,
  label,
  icon: Icon,
  requiresFiles = false,
}: {
  path: string
  label: string
  icon: LucideIcon
  requiresFiles?: boolean
}) {
  const location = useLocation()
  const currentPath = location.pathname
  const { open } = useSidebar()
  const binariesOk = useAppStore(state => state.binariesOk)
  const isActive = currentPath === path
  const isDisabled = requiresFiles && binariesOk === false

  const tooltipLabel = isDisabled
    ? `${label} недоступны, пока файлы приложения или фильтры отсутствуют`
    : label

  const content = (
    <>
      <span className="flex size-7 shrink-0 items-center justify-center">
        <Icon className="size-4 shrink-0" />
      </span>
      <span className="min-w-0 flex-1 overflow-hidden">
        <span
          className={cn(
            'block whitespace-nowrap text-left transition-all duration-200 ease-out',
            open ? 'translate-x-0 opacity-100 delay-75' : '-translate-x-2 opacity-0',
          )}
          aria-hidden={!open}
        >
          {label}
        </span>
      </span>
    </>
  )

  if (isDisabled) {
    return (
      <SidebarMenuItem>
        <SidebarMenuButton
          isActive={false}
          aria-label={open ? label : tooltipLabel}
          aria-disabled="true"
          tabIndex={0}
          className="cursor-not-allowed opacity-45 hover:bg-transparent hover:text-sidebar-foreground"
          onClick={(event) => {
            event.preventDefault()
          }}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault()
            }
          }}
        >
          {content}
        </SidebarMenuButton>
      </SidebarMenuItem>
    )
  }

  const link = (
    <Link
      to={path}
      aria-label={open ? label : tooltipLabel}
      className="flex w-full items-center overflow-hidden"
    >
      {content}
    </Link>
  )

  return (
    <SidebarMenuItem>
      <Tooltip open={open ? false : undefined}>
        <TooltipTrigger asChild>
          <SidebarMenuButton asChild isActive={isActive}>
            {link}
          </SidebarMenuButton>
        </TooltipTrigger>
        <TooltipContent side="right">
          {tooltipLabel}
        </TooltipContent>
      </Tooltip>
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
    <AppSidebar className="h-full bg-transparent">
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
