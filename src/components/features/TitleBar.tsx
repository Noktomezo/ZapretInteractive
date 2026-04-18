import { Link, useLocation } from '@tanstack/react-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Fragment } from 'react'
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from '@/components/ui/breadcrumb'
import { SidebarTrigger } from '@/components/ui/sidebar'
import { cn } from '@/lib/utils'
import { useConfigStore } from '@/stores/config.store'

interface BreadcrumbEntry {
  label: string
  to?: string
}

function safeDecode(value: string) {
  try {
    return decodeURIComponent(value)
  }
  catch {
    return value
  }
}

function getBreadcrumbItems(pathname: string, categoryName?: string): BreadcrumbEntry[] {
  if (pathname === '/') {
    return [{ label: 'Главная' }]
  }

  if (pathname === '/about') {
    return [{ label: 'О программе' }]
  }

  if (pathname === '/settings') {
    return [{ label: 'Настройки' }]
  }

  if (pathname === '/logs') {
    return [{ label: 'Логи' }]
  }

  if (pathname === '/filters') {
    return [{ label: 'Фильтры' }]
  }

  if (pathname === '/placeholders') {
    return [{ label: 'Плейсхолдеры' }]
  }

  if (pathname === '/strategies') {
    return [{ label: 'Стратегии' }]
  }

  if (pathname.startsWith('/strategies/')) {
    return [
      { label: 'Стратегии', to: '/strategies' },
      { label: categoryName ?? 'Категория' },
    ]
  }

  if (pathname === '/modules') {
    return [{ label: 'Модули' }]
  }

  if (pathname === '/modules/dns') {
    return [
      { label: 'Модули', to: '/modules' },
      { label: 'DNS' },
    ]
  }

  if (pathname === '/modules/tg-ws-proxy') {
    return [
      { label: 'Модули', to: '/modules' },
      { label: 'TG WS Proxy' },
    ]
  }

  return []
}

export function TitleBar() {
  const location = useLocation()
  const handleMinimize = () => getCurrentWindow().minimize()
  const handleClose = () => getCurrentWindow().close()
  const config = useConfigStore(state => state.config)
  const windowMaterial = config?.windowMaterial ?? 'acrylic'
  const materialEnabled = windowMaterial !== 'none'
  const currentCategoryName = location.pathname.startsWith('/strategies/')
    ? config?.categories.find(category => category.id === safeDecode(location.pathname.slice('/strategies/'.length)))?.name
    : undefined
  const breadcrumbItems = getBreadcrumbItems(location.pathname, currentCategoryName)

  return (
    <header
      className={cn(
        'fixed top-0 left-0 right-0 z-50 flex h-[var(--titlebar-height)] cursor-grab items-center px-2.5 select-none active:cursor-grabbing',
        materialEnabled ? 'bg-transparent' : 'bg-background',
      )}
      data-tauri-drag-region
      data-no-select="true"
    >
      <SidebarTrigger className="ml-[3px] mr-1 size-[30px]" />
      <div
        className="pointer-events-none absolute inset-0 flex items-center justify-center px-16"
        data-tauri-drag-region
      >
        {breadcrumbItems.length > 0 && (
          <Breadcrumb className="pointer-events-auto max-w-[min(42rem,calc(100vw-10rem))]">
            <BreadcrumbList className="flex-nowrap justify-center overflow-hidden whitespace-nowrap">
              {breadcrumbItems.map((item, index) => {
                const isLast = index === breadcrumbItems.length - 1

                return (
                  <Fragment key={`${item.label}-${index}`}>
                    <BreadcrumbItem className="min-w-0 shrink truncate">
                      {isLast
                        ? (
                            <BreadcrumbPage className="block truncate text-xs">
                              {item.label}
                            </BreadcrumbPage>
                          )
                        : (
                            <BreadcrumbLink asChild className="block truncate text-xs text-muted-foreground hover:text-foreground">
                              <Link to={item.to!} className="cursor-pointer">
                                {item.label}
                              </Link>
                            </BreadcrumbLink>
                          )}
                    </BreadcrumbItem>
                    {!isLast && <BreadcrumbSeparator className="shrink-0" />}
                  </Fragment>
                )
              })}
            </BreadcrumbList>
          </Breadcrumb>
        )}
      </div>
      <div className="flex-1" data-tauri-drag-region />
      <div className="-mr-2.5 flex h-full items-stretch">
        <button
          type="button"
          aria-label="Minimize"
          onClick={handleMinimize}
          className="flex h-full w-[46px] cursor-pointer items-center justify-center text-foreground/78 transition-colors hover:bg-black/6 hover:text-foreground dark:hover:bg-white/8"
          title="Minimize"
        >
          <svg aria-hidden="true" className="size-3" viewBox="0 0 10 10" fill="none">
            <path d="M1 5.5H9" stroke="currentColor" strokeWidth="1.1" strokeLinecap="square" />
          </svg>
        </button>
        <button
          type="button"
          aria-label="Close"
          onClick={handleClose}
          className="flex h-full w-[46px] cursor-pointer items-center justify-center text-foreground/78 transition-colors hover:bg-red-500/92 hover:text-white"
          title="Close"
        >
          <svg aria-hidden="true" className="size-3" viewBox="0 0 10 10" fill="none">
            <path d="M2 2L8 8M8 2L2 8" stroke="currentColor" strokeWidth="1.1" strokeLinecap="square" />
          </svg>
        </button>
      </div>
    </header>
  )
}
