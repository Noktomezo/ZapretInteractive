import type { LucideIcon } from 'lucide-react'
import { Link, useLocation } from '@tanstack/react-router'
import {
  Boxes,
  FileCode,
  Filter,
  Home,
  Info,
  Layers,
  Logs,
  Settings,
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
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

interface NavItemConfig {
  path: string
  labelKey: 'sidebar.home' | 'sidebar.modules' | 'sidebar.categories' | 'sidebar.filters' | 'sidebar.placeholders' | 'sidebar.logs' | 'sidebar.settings' | 'sidebar.about'
  icon: LucideIcon
  requiresFiles?: boolean
}

const mainNavItems: NavItemConfig[] = [
  { path: '/', labelKey: 'sidebar.home', icon: Home },
  { path: '/modules', labelKey: 'sidebar.modules', icon: Boxes },
  { path: '/strategies', labelKey: 'sidebar.categories', icon: Layers, requiresFiles: true },
  { path: '/filters', labelKey: 'sidebar.filters', icon: Filter, requiresFiles: true },
  { path: '/placeholders', labelKey: 'sidebar.placeholders', icon: FileCode, requiresFiles: true },
  { path: '/logs', labelKey: 'sidebar.logs', icon: Logs },
]

const footerNavItems: NavItemConfig[] = [
  { path: '/settings', labelKey: 'sidebar.settings', icon: Settings },
  { path: '/about', labelKey: 'sidebar.about', icon: Info },
]

function SidebarNavItem({
  path,
  labelKey,
  icon: Icon,
  requiresFiles = false,
}: NavItemConfig) {
  const { t } = useTranslation()
  const label = t(labelKey)
  const location = useLocation()
  const currentPath = location.pathname
  const { open } = useSidebar()
  const binariesOk = useAppStore(state => state.binariesOk)
  const isActive = path === '/'
    ? currentPath === path
    : currentPath === path || currentPath.startsWith(`${path}/`)
  const isDisabled = requiresFiles && binariesOk === false

  const tooltipLabel = isDisabled
    ? `${label} недоступны, пока файлы приложения или фильтры отсутствуют`
    : label

  const content = (
    <>
      <span className="flex size-6 shrink-0 items-center justify-center">
        <Icon className="size-[0.9rem] shrink-0" />
      </span>
      <span
        className={cn(
          'min-w-0 overflow-hidden transition-[max-width,margin,opacity] duration-200 ease-out',
          open ? 'ml-1.5 max-w-40 flex-1 opacity-100' : 'ml-0 max-w-0 opacity-0',
        )}
      >
        <span
          className={cn(
            'block whitespace-nowrap text-left',
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
        <Tooltip open={open ? false : undefined}>
          <TooltipTrigger asChild>
            <SidebarMenuButton
              isActive={false}
              aria-label={tooltipLabel}
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
          </TooltipTrigger>
          <TooltipContent side="right">{tooltipLabel}</TooltipContent>
        </Tooltip>
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

export function AppNavigationSidebar() {
  return (
    <AppSidebar className="h-full">
      <SidebarContent>
        <SidebarNav />
      </SidebarContent>
      <SidebarFooter>
        <SidebarMenu>
          {footerNavItems.map(item => (
            <SidebarNavItem key={item.path} {...item} />
          ))}
        </SidebarMenu>
      </SidebarFooter>
      <SidebarRail />
    </AppSidebar>
  )
}
