import { ArrowDown, BrushCleaning } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { Button } from '@/components/ui/button'
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import { useConnectionStore } from '@/stores/connection.store'

const logTimestampFormatter = new Intl.DateTimeFormat('ru-RU', {
  day: '2-digit',
  month: '2-digit',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
})

function formatLogTimestamp(timestamp: number) {
  return logTimestampFormatter.format(new Date(timestamp))
}

export function LogsPage() {
  const logs = useConnectionStore(state => state.logs)
  const clearLogs = useConnectionStore(state => state.clearLogs)
  const scrollAreaRef = useRef<HTMLDivElement | null>(null)
  const [isPinnedToBottom, setIsPinnedToBottom] = useState(true)
  const [hasUnreadLogs, setHasUnreadLogs] = useState(false)

  const getViewport = () => {
    return scrollAreaRef.current?.querySelector('[data-slot="lenis-scroll-area-viewport"], [data-slot="scroll-area-viewport"]') as HTMLDivElement | null
  }

  const getLenis = () => {
    const viewport = getViewport()
    return (viewport as (HTMLDivElement & { __lenis?: { scrollTo: (target: number, options?: { duration?: number, easing?: (value: number) => number }) => void } }) | null)?.__lenis ?? null
  }

  useEffect(() => {
    const viewport = getViewport()
    if (!viewport)
      return

    const handleScroll = () => {
      const nearBottom = viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight < 24
      setIsPinnedToBottom(nearBottom)
      if (nearBottom)
        setHasUnreadLogs(false)
    }

    handleScroll()
    viewport.addEventListener('scroll', handleScroll)
    return () => {
      viewport.removeEventListener('scroll', handleScroll)
    }
  }, [])

  useEffect(() => {
    const viewport = getViewport()
    if (!viewport)
      return

    if (logs.length === 0) {
      setHasUnreadLogs(false)
      return
    }

    if (isPinnedToBottom) {
      viewport.scrollTo({ top: viewport.scrollHeight })
      setHasUnreadLogs(false)
    }
    else if (logs.length > 0) {
      setHasUnreadLogs(true)
    }
  }, [isPinnedToBottom, logs])

  return (
    <div className="flex h-full min-h-0 flex-col p-6">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-medium">Логи</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Журнал запуска, остановки и внутренних событий подключения.
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          size="default"
          onClick={clearLogs}
          disabled={logs.length === 0}
          className="gap-2"
        >
          <BrushCleaning className="text-warning h-4 w-4" />
          Очистить
        </Button>
      </div>

      <div className="relative mt-6 min-h-0 flex-1 overflow-hidden rounded-xl border border-border bg-card">
        {hasUnreadLogs && (
          <Button
            type="button"
            size="sm"
            variant="secondary"
            className="absolute right-4 bottom-4 z-10 border border-border bg-background/60 text-foreground shadow-lg backdrop-blur-md hover:bg-background/72 hover:backdrop-blur-xl dark:bg-card/60 dark:hover:bg-card/72"
            onClick={() => {
              const viewport = getViewport()
              if (!viewport)
                return
              const lenis = getLenis()
              if (lenis) {
                lenis.scrollTo(viewport.scrollHeight, {
                  duration: 0.45,
                  easing: value => 1 - (1 - value) ** 3,
                })
              }
              else {
                viewport.scrollTo({ top: viewport.scrollHeight, behavior: 'smooth' })
              }
              setIsPinnedToBottom(true)
              setHasUnreadLogs(false)
            }}
          >
            <ArrowDown className="h-4 w-4" />
            Новые логи
          </Button>
        )}
        <LenisScrollArea ref={scrollAreaRef} className="h-full">
          <div className="space-y-1 p-4 font-mono text-xs text-foreground">
            {logs.length === 0
              ? (
                  <div className="text-muted-foreground flex min-h-[12rem] items-center justify-center">
                    Нет логов
                  </div>
                )
              : (
                  logs.map(log => (
                    <p key={log.seq} className="text-foreground whitespace-pre-wrap">
                      <span className="text-muted-foreground">
                        [
                        {formatLogTimestamp(log.timestamp)}
                        ]
                      </span>
                      <span> </span>
                      <span>{log.message}</span>
                    </p>
                  ))
                )}
          </div>
        </LenisScrollArea>
      </div>
    </div>
  )
}
