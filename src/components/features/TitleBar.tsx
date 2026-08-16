import type { ReactNode } from 'react'
import { Link, useLocation } from '@tanstack/react-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Copy, Minus, Square, X } from 'lucide-react'
import { Fragment, useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
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
  id: string
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

function WindowControlButton({
  label,
  onClick,
  destructive = false,
  children,
}: {
  label: string
  onClick: () => void
  destructive?: boolean
  children: ReactNode
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className={cn(
        'inline-flex size-[30px] cursor-pointer items-center justify-center rounded-md bg-transparent text-foreground/82 transition-colors hover:bg-accent/70 hover:text-foreground',
        destructive && 'hover:bg-destructive/15 hover:text-destructive',
      )}
      title={label}
    >
      {children}
    </button>
  )
}

export function TitleBar() {
  const { t } = useTranslation()
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
  const currentCategoryName = location.pathname.startsWith('/strategies/')
    ? config?.categories.find(category => category.id === safeDecode(location.pathname.slice('/strategies/'.length)))?.name
    : undefined

  const getBreadcrumbs = (pathname: string): BreadcrumbEntry[] => {
    if (pathname === '/') {
      return [{ id: 'home', label: t('sidebar.home') }]
    }
    if (pathname === '/about') {
      return [{ id: 'about', label: t('sidebar.about') }]
    }
    if (pathname === '/settings') {
      return [{ id: 'settings', label: t('sidebar.settings') }]
    }
    if (pathname === '/logs') {
      return [{ id: 'logs', label: t('sidebar.logs') }]
    }
    if (pathname === '/filters') {
      return [{ id: 'filters', label: t('sidebar.filters') }]
    }
    if (pathname === '/placeholders') {
      return [{ id: 'placeholders', label: t('sidebar.placeholders') }]
    }
    if (pathname === '/strategies') {
      return [{ id: 'strategies', label: t('sidebar.categories') }]
    }
    if (pathname.startsWith('/strategies/')) {
      return [
        { id: 'strategies-root', label: t('sidebar.categories'), to: '/strategies' },
        { id: `category-${pathname.split('/').pop() || 'category'}`, label: currentCategoryName ?? t('categories.title') },
      ]
    }
    if (pathname === '/modules') {
      return [{ id: 'modules', label: t('sidebar.modules') }]
    }
    if (pathname === '/modules/dns') {
      return [
        { id: 'modules-root', label: t('sidebar.modules'), to: '/modules' },
        { id: 'dns', label: t('modules.dns.title') },
      ]
    }
    if (pathname === '/modules/tg-ws-proxy') {
      return [
        { id: 'modules-root', label: t('sidebar.modules'), to: '/modules' },
        { id: 'tg-ws-proxy', label: t('modules.tgWsProxy.title') },
      ]
    }
    return []
  }

  const breadcrumbItems = getBreadcrumbs(location.pathname)
  const maximizeLabel = isMaximized ? t('titleBar.restore') : t('titleBar.maximize')

  useEffect(() => {
    let unlisten: (() => void) | undefined
    let mounted = true
    let syncAnimationFrame = 0
    let syncScheduled = false
    const appWindow = getCurrentWindow()

    const scheduleMaximizedStateSync = () => {
      if (syncScheduled) {
        return
      }

      syncScheduled = true
      syncAnimationFrame = window.requestAnimationFrame(() => {
        syncScheduled = false
        if (mounted) {
          syncMaximizedState()
        }
      })
    }

    syncMaximizedState()
    void appWindow.onResized(() => {
      if (mounted) {
        scheduleMaximizedStateSync()
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
      if (syncAnimationFrame) {
        window.cancelAnimationFrame(syncAnimationFrame)
      }
      unlisten?.()
    }
  }, [syncMaximizedState])

  return (
    <header
      className="fixed top-0 left-0 right-0 z-50 flex h-[var(--titlebar-height)] cursor-grab items-center px-[5px] select-none active:cursor-grabbing bg-background"
      data-tauri-drag-region
      data-no-select="true"
    >
      <SidebarTrigger className="mr-1 size-[30px]" />
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
                  <Fragment key={item.id}>
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
      <div className="flex items-center gap-1" data-tauri-drag-region>
        <WindowControlButton label={t('titleBar.minimize')} onClick={handleMinimize}>
          <Minus aria-hidden="true" className="size-3.5" strokeWidth={2} />
        </WindowControlButton>
        <WindowControlButton label={maximizeLabel} onClick={handleToggleMaximize}>
          {isMaximized
            ? <Copy aria-hidden="true" className="size-3.5" strokeWidth={2} />
            : <Square aria-hidden="true" className="size-3.5" strokeWidth={2} />}
        </WindowControlButton>
        <WindowControlButton label={t('titleBar.close')} onClick={handleClose} destructive>
          <X aria-hidden="true" className="size-3.5" strokeWidth={2} />
        </WindowControlButton>
      </div>
    </header>
  )
}
