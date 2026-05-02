import { Link, useLocation } from '@tanstack/react-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Copy, Minus, Square, X } from 'lucide-react'
import { Fragment, useCallback, useEffect, useState } from 'react'
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
  const [isMaximized, setIsMaximized] = useState(false)
  const syncMaximizedState = useCallback(() => {
    void getCurrentWindow().isMaximized().then(setIsMaximized).catch(console.error)
  }, [])
  const handleMinimize = () => void getCurrentWindow().minimize().catch(console.error)
  const handleToggleMaximize = () => {
    void getCurrentWindow().toggleMaximize().then(syncMaximizedState).catch(console.error)
  }
  const handleClose = () => void getCurrentWindow().close().catch(console.error)
  const config = useConfigStore(state => state.config)
  const windowMaterial = config?.windowMaterial ?? 'none'
  const materialEnabled = windowMaterial !== 'none'
  const currentCategoryName = location.pathname.startsWith('/strategies/')
    ? config?.categories.find(category => category.id === safeDecode(location.pathname.slice('/strategies/'.length)))?.name
    : undefined
  const breadcrumbItems = getBreadcrumbItems(location.pathname, currentCategoryName)

  useEffect(() => {
    let unlisten: (() => void) | undefined
    let mounted = true
    const appWindow = getCurrentWindow()

    syncMaximizedState()
    void appWindow.onResized(() => {
      if (mounted) {
        syncMaximizedState()
      }
    }).then((cleanup) => {
      if (mounted) {
        unlisten = cleanup
        return
      }

      cleanup()
    }).catch(console.error)

    return () => {
      mounted = false
      unlisten?.()
    }
  }, [syncMaximizedState])

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
          className="flex h-full w-[46px] cursor-pointer items-center justify-center text-foreground/78 transition-colors hover:bg-accent/70 hover:text-foreground"
          title="Minimize"
        >
          <Minus aria-hidden="true" className="size-3.5" strokeWidth={2} />
        </button>
        <button
          type="button"
          aria-label={isMaximized ? 'Restore' : 'Maximize'}
          onClick={handleToggleMaximize}
          className="flex h-full w-[46px] cursor-pointer items-center justify-center text-foreground/78 transition-colors hover:bg-accent/70 hover:text-foreground"
          title={isMaximized ? 'Restore' : 'Maximize'}
        >
          {isMaximized
            ? <Copy aria-hidden="true" className="size-3.5" strokeWidth={2} />
            : <Square aria-hidden="true" className="size-3.5" strokeWidth={2} />}
        </button>
        <button
          type="button"
          aria-label="Close"
          onClick={handleClose}
          className="flex h-full w-[46px] cursor-pointer items-center justify-center text-foreground/78 transition-colors hover:bg-destructive/88 hover:text-destructive-foreground dark:hover:bg-destructive/72"
          title="Close"
        >
          <X aria-hidden="true" className="size-3.5" strokeWidth={2} />
        </button>
      </div>
    </header>
  )
}
