import { useState, useEffect } from 'react'
import { Power, ChevronDown, ChevronUp, AlertCircle, Loader2, Trash2 } from 'lucide-react'
import { listen } from '@tauri-apps/api/event'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'
import { useDownloadStore } from '@/stores/download.store'
import { useAppStore } from '@/stores/app.store'
import * as tauri from '@/lib/tauri'
import { cn } from '@/lib/utils'

interface DownloadProgress {
  current: number
  total: number
  filename: string
  phase: 'binaries' | 'fake' | 'lists'
}

export function MainPage() {
  const [logsOpen, setLogsOpen] = useState(false)

  const { config } = useConfigStore()
  const { status, connect, disconnect, logs, clearLogs } = useConnectionStore()
  const { isDownloading, progress, setDownloading, setProgress, reset } = useDownloadStore()
  const { initialized, isElevated, binariesOk, initialize, setBinariesOk } = useAppStore()

  useEffect(() => {
    const unlistenStart = listen('download-start', () => {
      setDownloading(true)
    })

    const unlistenProgress = listen<DownloadProgress>('download-progress', (event) => {
      setProgress(event.payload)
    })

    const unlistenComplete = listen('download-complete', async () => {
      reset()
      const binaries = await tauri.verifyBinaries()
      setBinariesOk(binaries)
    })

    const unlistenError = listen<string>('download-error', () => {
      reset()
    })

    return () => {
      unlistenStart.then(fn => fn())
      unlistenProgress.then(fn => fn())
      unlistenComplete.then(fn => fn())
      unlistenError.then(fn => fn())
    }
  }, [])

  useEffect(() => {
    initialize()
  }, [])

  const handleToggleConnection = async () => {
    if (status === 'connected') {
      await disconnect()
    } else {
      await connect()
    }
    await useConfigStore.getState().save()
  }

  const handleDownloadBinaries = async () => {
    setDownloading(true)
    try {
      await tauri.downloadBinaries()
    } catch (e) {
      console.error(e)
      reset()
    }
  }

  if (!initialized) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="w-6 h-6 animate-spin" />
      </div>
    )
  }

  if (!isElevated) {
    return (
      <div className="flex items-center justify-center h-full p-8">
        <Alert variant="destructive" className="max-w-lg">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>Требуются права администратора</AlertTitle>
          <AlertDescription>
            Для работы WinDivert необходимы права администратора. 
            Запустите приложение от имени администратора.
          </AlertDescription>
        </Alert>
      </div>
    )
  }

  if (binariesOk === false) {
    return (
      <div className="flex items-center justify-center h-full p-8">
        <Card className="p-6 max-w-md text-center space-y-4">
          <div className="w-16 h-16 mx-auto rounded-full bg-accent flex items-center justify-center">
            <AlertCircle className="w-8 h-8 text-muted-foreground" />
          </div>
          <div>
            <h2 className="text-lg font-semibold">Требуется загрузка</h2>
            <p className="text-sm text-muted-foreground mt-1">
              Необходимые файлы не найдены. Нажмите кнопку для загрузки.
            </p>
          </div>
          {isDownloading && progress ? (
            <div className="space-y-2">
              <div className="h-2 bg-muted rounded-full overflow-hidden">
                <div 
                  className="h-full bg-primary transition-all duration-300"
                  style={{ width: `${(progress.current / progress.total) * 100}%` }}
                />
              </div>
              <p className="text-xs text-muted-foreground">
                {progress.current}/{progress.total}: {progress.filename}
              </p>
            </div>
          ) : (
            <Button onClick={handleDownloadBinaries} className="w-full">
              Загрузить файлы
            </Button>
          )}
        </Card>
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 flex items-center justify-center p-8">
        <div className="text-center space-y-6">
          <div className="relative">
            <Button
              onClick={handleToggleConnection}
              disabled={status === 'connecting' || status === 'disconnecting' || !config}
              variant="ghost"
              className={cn(
                'w-32 h-32 rounded-full transition-all duration-300',
                status === 'connected' && 'bg-green-600 hover:bg-green-500 dark:hover:bg-green-500 animate-pulse-glow text-white',
                status === 'connecting' && 'bg-yellow-600 hover:bg-yellow-500 text-white',
                status === 'disconnecting' && 'bg-orange-600 hover:bg-orange-500 text-white',
                status === 'error' && 'bg-red-600 hover:bg-red-500 text-white',
                status === 'disconnected' && 'bg-neutral-900 hover:bg-neutral-800 dark:bg-neutral-100 dark:hover:bg-neutral-200 animate-pulse-glow-primary text-white dark:text-neutral-900'
              )}
            >
              <Power className="size-12" />
            </Button>
            {(status === 'connecting' || status === 'disconnecting') && (
              <div className="absolute inset-0 rounded-full animate-ping bg-yellow-500/30" />
            )}
          </div>

          <div>
            <h2 className="text-2xl font-semibold">
              {status === 'connected' ? 'Подключено' : 
               status === 'connecting' ? 'Подключение...' :
               status === 'disconnecting' ? 'Отключение...' :
               status === 'error' ? 'Ошибка' : 'Отключено'}
            </h2>
          </div>
        </div>
      </div>

      <div className="border-t border-border">
        <button
          onClick={() => setLogsOpen(!logsOpen)}
          className="w-full px-4 py-2 flex items-center justify-between text-sm hover:bg-accent transition-colors cursor-pointer"
        >
          <span className="text-muted-foreground">Логи</span>
          <div className="flex items-center gap-2">
            {logsOpen && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Trash2
                    onClick={(e) => { e.stopPropagation(); clearLogs(); }}
                    className="w-4 h-4 text-muted-foreground hover:text-yellow-500 transition-colors cursor-pointer"
                  />
                </TooltipTrigger>
                <TooltipContent>Очистить логи</TooltipContent>
              </Tooltip>
            )}
            {logsOpen ? <ChevronDown className="w-4 h-4" /> : <ChevronUp className="w-4 h-4" />}
          </div>
        </button>

        {logsOpen && (
          <div className="border-t border-border">
            <ScrollArea className="h-48">
              <div className="p-3 font-mono text-xs space-y-1">
                {logs.length === 0 ? (
                  <p className="text-muted-foreground">Нет логов</p>
                ) : (
                  logs.map((log, i) => (
                    <p key={i} className="text-muted-foreground">
                      {log}
                    </p>
                  ))
                )}
              </div>
            </ScrollArea>
          </div>
        )}
      </div>
    </div>
  )
}