import { PanelLeftClose, PanelLeftOpen } from 'lucide-react'
import * as React from 'react'
import { cn } from '@/lib/utils'

interface SidebarContextValue {
  open: boolean
  setOpen: (open: boolean) => void
  toggleSidebar: () => void
}

const SidebarContext = React.createContext<SidebarContextValue | null>(null)

function useSidebar() {
  const context = React.useContext(SidebarContext)

  if (!context)
    throw new Error('useSidebar must be used within a SidebarProvider.')

  return context
}

function composeEventHandlers<E>(
  first?: (event: E) => void,
  second?: (event: E) => void,
) {
  return (event: E) => {
    first?.(event)
    second?.(event)
  }
}

function SidebarProvider({
  defaultOpen = true,
  open: openProp,
  onOpenChange,
  className,
  style,
  children,
  ...props
}: React.ComponentProps<'div'> & {
  defaultOpen?: boolean
  open?: boolean
  onOpenChange?: (open: boolean) => void
}) {
  const [openState, setOpenState] = React.useState(defaultOpen)

  const open = openProp ?? openState
  const setOpen = React.useCallback((value: boolean) => {
    onOpenChange?.(value)

    if (openProp === undefined)
      setOpenState(value)
  }, [onOpenChange, openProp])

  const toggleSidebar = React.useCallback(() => {
    setOpen(!open)
  }, [open, setOpen])

  return (
    <SidebarContext.Provider value={{ open, setOpen, toggleSidebar }}>
      <div
        data-slot="sidebar-provider"
        style={{
          '--sidebar-width': '13rem',
          '--sidebar-width-icon': '3.5rem',
          ...style,
        } as React.CSSProperties}
        className={cn('group/sidebar-wrapper flex h-full min-h-0 w-full', className)}
        {...props}
      >
        {children}
      </div>
    </SidebarContext.Provider>
  )
}

function Sidebar({ className, children, ...props }: React.ComponentProps<'aside'>) {
  const { open } = useSidebar()

  return (
    <aside
      data-slot="sidebar"
      data-state={open ? 'expanded' : 'collapsed'}
      className={cn(
        'relative hidden h-full shrink-0 bg-transparent text-sidebar-foreground transition-[width] duration-250 ease-out md:flex',
        open ? 'w-[var(--sidebar-width)]' : 'w-[var(--sidebar-width-icon)]',
        className,
      )}
      {...props}
    >
      <div className="flex min-h-0 w-full flex-col">{children}</div>
    </aside>
  )
}

function SidebarHeader({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot="sidebar-header" className={cn('flex items-center gap-2 border-b border-sidebar-border p-2', className)} {...props} />
}

function SidebarContent({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot="sidebar-content" className={cn('flex min-h-0 flex-1 flex-col gap-2 overflow-auto p-2', className)} {...props} />
}

function SidebarFooter({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot="sidebar-footer" className={cn('mt-auto border-t border-sidebar-border p-2', className)} {...props} />
}

function SidebarGroup({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot="sidebar-group" className={cn('flex w-full min-w-0 flex-col', className)} {...props} />
}

function SidebarGroupContent({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot="sidebar-group-content" className={cn('w-full text-sm', className)} {...props} />
}

function SidebarMenu({ className, ...props }: React.ComponentProps<'ul'>) {
  return <ul data-slot="sidebar-menu" className={cn('flex w-full min-w-0 flex-col gap-1', className)} {...props} />
}

function SidebarMenuItem({ className, ...props }: React.ComponentProps<'li'>) {
  return <li data-slot="sidebar-menu-item" className={cn('group/menu-item relative', className)} {...props} />
}

const SidebarMenuButton = React.forwardRef<React.ElementRef<'button'>, React.ComponentProps<'button'> & {
  isActive?: boolean
  asChild?: boolean
}>(({
  className,
  isActive = false,
  asChild = false,
  children,
  type = 'button',
  onClick,
  ...props
}, ref) => {
  const classes = cn(
    'flex h-10 w-full items-center overflow-hidden rounded-lg px-1.5 text-left text-sm font-normal outline-none transition-[background-color,color,box-shadow] duration-200 ease-out',
    'text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground',
    'data-[active=true]:bg-sidebar-primary data-[active=true]:text-sidebar-primary-foreground data-[active=true]:shadow-sm',
    className,
  )

  if (asChild && React.isValidElement(children)) {
    const child = children as React.ReactElement<any>
    const isButton = child.type === 'button' || child.type === 'input'

    return React.cloneElement(child, {
      ...props,
      ...child.props,
      ref,
      ...(isButton && { type }),
      'onClick': composeEventHandlers(onClick, child.props.onClick),
      'className': cn(classes, child.props.className),
      'data-slot': 'sidebar-menu-button',
      'data-active': isActive,
    })
  }

  return (
    <button
      type={type}
      data-slot="sidebar-menu-button"
      data-active={isActive}
      className={classes}
      onClick={onClick}
      ref={ref}
      {...props}
    >
      {children}
    </button>
  )
})
SidebarMenuButton.displayName = 'SidebarMenuButton'

function SidebarTrigger({ className, onClick, ...props }: React.ComponentProps<'button'>) {
  const { open, toggleSidebar } = useSidebar()
  const Icon = open ? PanelLeftClose : PanelLeftOpen

  return (
    <button
      data-slot="sidebar-trigger"
      type="button"
      className={cn('inline-flex size-9 cursor-pointer items-center justify-center rounded-md text-foreground transition-colors hover:text-foreground', className)}
      onClick={composeEventHandlers(onClick, (event) => {
        if (!event.defaultPrevented)
          toggleSidebar()
      })}
      {...props}
    >
      <Icon className="size-4" aria-hidden="true" />
      <span className="sr-only">{open ? 'Collapse Sidebar' : 'Expand Sidebar'}</span>
    </button>
  )
}

function SidebarRail({ className, onClick, ...props }: React.ComponentProps<'button'>) {
  const { open, toggleSidebar } = useSidebar()

  return (
    <button
      data-slot="sidebar-rail"
      type="button"
      aria-label="Toggle Sidebar"
      onClick={composeEventHandlers(onClick, (event) => {
        if (!event.defaultPrevented)
          toggleSidebar()
      })}
      className={cn('absolute inset-y-0 -right-px hidden w-px cursor-ew-resize bg-transparent opacity-0 transition-opacity group-hover/sidebar-wrapper:opacity-100 md:block', className)}
      {...props}
    >
      <span className="sr-only">{open ? 'Collapse Sidebar' : 'Expand Sidebar'}</span>
    </button>
  )
}

function SidebarInset({ className, ...props }: React.ComponentProps<'main'>) {
  return <main data-slot="sidebar-inset" className={cn('flex min-w-0 flex-1 flex-col bg-background', className)} {...props} />
}

export {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarRail,
  SidebarTrigger,
  useSidebar,
}
