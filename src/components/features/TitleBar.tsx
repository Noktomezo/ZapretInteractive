import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEffect, useState } from 'react'
import { SidebarTrigger } from '@/components/ui/sidebar'

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false)

  useEffect(() => {
    const appWindow = getCurrentWindow()
    let resizeTimer: ReturnType<typeof setTimeout> | null = null

    const syncMaximizedState = async () => {
      try {
        const maximized = await appWindow.isMaximized()
        setIsMaximized(maximized)
      }
      catch (error) {
        console.error(error)
      }
    }

    syncMaximizedState()
    const unlisten = appWindow.onResized(async () => {
      if (resizeTimer)
        clearTimeout(resizeTimer)

      resizeTimer = setTimeout(() => {
        void syncMaximizedState()
      }, 150)
    })
    return () => {
      if (resizeTimer)
        clearTimeout(resizeTimer)
      unlisten.then(fn => fn()).catch(console.error)
    }
  }, [])

  const handleMinimize = () => getCurrentWindow().minimize()
  const handleMaximize = () => getCurrentWindow().toggleMaximize()
  const handleClose = () => getCurrentWindow().close()

  return (
    <header
      className="fixed top-0 left-0 right-0 z-50 flex h-10 items-center border-b border-border bg-card px-3 select-none"
      data-tauri-drag-region
    >
      <SidebarTrigger className="mr-2 size-9" />
      <div className="flex-1" data-tauri-drag-region />
      <div className="flex items-center gap-2">
        <button
          type="button"
          aria-label="Minimize"
          onClick={handleMinimize}
          className="group flex size-4 items-center justify-center rounded-full bg-yellow-500/90 transition-colors hover:bg-yellow-400"
          title="Minimize"
        >
          <svg aria-hidden="true" className="size-2.5 text-yellow-950 opacity-0 group-hover:opacity-100" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 13H5v-2h14z" />
          </svg>
        </button>
        <button
          type="button"
          aria-label={isMaximized ? 'Restore' : 'Maximize'}
          onClick={handleMaximize}
          className="group flex size-4 items-center justify-center rounded-full bg-green-500/90 transition-colors hover:bg-green-400"
          title={isMaximized ? 'Restore' : 'Maximize'}
        >
          {isMaximized
            ? (
                <svg aria-hidden="true" className="size-2.5 text-green-950 opacity-0 group-hover:opacity-100" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M5 16h3v3h2v-5H5v2zm3-8H5v2h5V5H8v3zm6 11h2v-3h3v-2h-5v5zm2-11V5h-2v5h5V8h-3z" />
                </svg>
              )
            : (
                <svg aria-hidden="true" className="size-2.5 text-green-950 opacity-0 group-hover:opacity-100" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M4 4h16v16H4zm2 4v10h12V8z" />
                </svg>
              )}
        </button>
        <button
          type="button"
          aria-label="Close"
          onClick={handleClose}
          className="group flex size-4 items-center justify-center rounded-full bg-red-500/90 transition-colors hover:bg-red-400"
          title="Close"
        >
          <svg aria-hidden="true" className="size-2.5 text-red-950 opacity-0 group-hover:opacity-100" viewBox="0 0 24 24" fill="currentColor">
            <path d="M13.46 12L19 17.54V19h-1.46L12 13.46L6.46 19H5v-1.46L10.54 12L5 6.46V5h1.46L12 10.54L17.54 5H19v1.46z" />
          </svg>
        </button>
      </div>
    </header>
  )
}
