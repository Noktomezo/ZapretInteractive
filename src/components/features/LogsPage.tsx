import { ArrowDown, BrushCleaning } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
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

  useEffect(() => {
    const viewport = scrollAreaRef.current?.querySelector('[data-slot="scroll-area-viewport"]') as HTMLDivElement | null
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
    const viewport = scrollAreaRef.current?.querySelector('[data-slot="scroll-area-viewport"]') as HTMLDivElement | null
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
          <BrushCleaning className="h-4 w-4 text-orange-500 dark:text-orange-400" />
          Очистить
        </Button>
      </div>

      <div className="relative mt-6 min-h-0 flex-1 overflow-hidden rounded-xl border border-border bg-card">
        {hasUnreadLogs && (
          <Button
            type="button"
            size="sm"
            variant="secondary"
            className="absolute right-4 bottom-4 z-10 border border-border bg-background text-foreground shadow-lg hover:bg-background dark:bg-card dark:hover:bg-card"
            onClick={() => {
              const viewport = scrollAreaRef.current?.querySelector('[data-slot="scroll-area-viewport"]') as HTMLDivElement | null
              if (!viewport)
                return
              viewport.scrollTo({ top: viewport.scrollHeight })
              setIsPinnedToBottom(true)
              setHasUnreadLogs(false)
            }}
          >
            <ArrowDown className="h-4 w-4" />
            Новые логи
          </Button>
        )}
        <ScrollArea ref={scrollAreaRef} className="h-full">
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
        </ScrollArea>
      </div>
    </div>
  )
}
