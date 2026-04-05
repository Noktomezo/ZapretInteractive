import { getCurrentWindow } from '@tauri-apps/api/window'
import { SidebarTrigger } from '@/components/ui/sidebar'
import { cn } from '@/lib/utils'
import { useConfigStore } from '@/stores/config.store'

export function TitleBar() {
  const handleMinimize = () => getCurrentWindow().minimize()
  const handleClose = () => getCurrentWindow().close()
  const windowMaterial = useConfigStore(state => state.config?.windowMaterial ?? 'acrylic')
  const materialEnabled = windowMaterial !== 'none'

  return (
    <header
      className={cn(
        'fixed top-0 left-0 right-0 z-50 flex h-[40px] cursor-grab items-center px-2.5 select-none active:cursor-grabbing',
        materialEnabled ? 'bg-transparent' : 'bg-background',
      )}
      data-tauri-drag-region
    >
      <SidebarTrigger className="ml-[3px] mr-1 size-[30px]" />
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
