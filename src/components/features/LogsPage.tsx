import { Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { useConnectionStore } from '@/stores/connection.store'

function formatLogTimestamp(timestamp: number) {
  return new Intl.DateTimeFormat('ru-RU', {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  }).format(new Date(timestamp))
}

export function LogsPage() {
  const { logs, clearLogs } = useConnectionStore()

  return (
    <div className="flex h-full min-h-0 flex-col p-6">
      <div className="flex items-start justify-between gap-4 pb-6">
        <div className="space-y-1">
          <h1 className="text-3xl font-semibold tracking-tight">Логи</h1>
          <p className="text-sm text-muted-foreground">
            Журнал запуска, остановки и внутренних событий подключения.
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={clearLogs}
          disabled={logs.length === 0}
          className="gap-2"
        >
          <Trash2 className="h-4 w-4" />
          Очистить
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-hidden rounded-xl border border-zinc-800/80 bg-zinc-950/92">
        <ScrollArea className="h-full">
          <div className="space-y-1 p-4 font-mono text-xs text-zinc-100">
            {logs.length === 0
              ? (
                  <div className="flex min-h-[12rem] items-center justify-center text-zinc-400">
                    Нет логов
                  </div>
                )
              : (
                  logs.map((log, index) => (
                    <p key={`${log.timestamp}-${index}`} className="whitespace-pre-wrap text-zinc-300">
                      <span className="text-zinc-500">[{formatLogTimestamp(log.timestamp)}]</span>
                      {' '}
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
