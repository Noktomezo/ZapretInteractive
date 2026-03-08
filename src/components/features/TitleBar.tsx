import { getCurrentWindow } from '@tauri-apps/api/window'
import { SidebarTrigger } from '@/components/ui/sidebar'

export function TitleBar() {
  const handleMinimize = () => getCurrentWindow().minimize()
  const handleClose = () => getCurrentWindow().close()

  return (
    <header
      className="fixed top-0 left-0 right-0 z-50 flex h-10 cursor-grab items-center bg-transparent px-2.5 select-none active:cursor-grabbing"
      data-tauri-drag-region
    >
      <SidebarTrigger className="mr-2 size-9" />
      <div className="flex-1" data-tauri-drag-region />
      <div className="-mr-2.5 flex h-full items-stretch">
        <button
          type="button"
          aria-label="Minimize"
          onClick={handleMinimize}
          className="flex h-full w-11 cursor-pointer items-center justify-center text-foreground/85 transition-colors hover:bg-black/8 hover:text-foreground dark:hover:bg-white/10"
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
          className="flex h-full w-11 cursor-pointer items-center justify-center text-foreground/85 transition-colors hover:bg-red-500 hover:text-white"
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
