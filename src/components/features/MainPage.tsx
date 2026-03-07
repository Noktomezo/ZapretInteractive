import type { ListMode } from '@/lib/types'
import {
  AlertCircle,
  Loader2,
  Power,
} from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import * as tauri from '@/lib/tauri'
import { cn } from '@/lib/utils'
import { useAppStore } from '@/stores/app.store'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'
import { useDownloadStore } from '@/stores/download.store'

export function MainPage() {
  const missingFilesToastShownRef = useRef(false)
  const [initError, setInitError] = useState<string | null>(null)
  const { config, setListMode } = useConfigStore()
  const { status, connect, disconnect } = useConnectionStore()
  const { isDownloading, progress, reset }
    = useDownloadStore()
  const { initialized, isElevated, binariesOk, initialize }
    = useAppStore()

  const handleListModeChange = (value: string) => {
    if (value) {
      setListMode(value as ListMode)
      useConfigStore.getState().save()
    }
  }

  useEffect(() => {
    initialize().catch((error) => {
      console.error('Failed to initialize app:', error)
      setInitError(String(error))
    })

    const unlistenListMode = tauri.onListModeChanged((mode) => {
      setListMode(mode)
    })

    return () => {
      unlistenListMode()
    }
  }, [])

  useEffect(() => {
    if (initialized && isElevated && binariesOk === false && !missingFilesToastShownRef.current) {
      missingFilesToastShownRef.current = true
      toast.error('Файлы приложения или фильтры отсутствуют либо повреждены. Обновите их вручную.')
    }
  }, [initialized, isElevated, binariesOk])

  const handleToggleConnection = async () => {
    const attemptedAction = status === 'connected' ? 'отключение' : 'подключение'
    try {
      if (status === 'connected') {
        await disconnect()
      }
      else {
        await connect()
      }
      await useConfigStore.getState().save()
    }
    catch (e) {
      toast.error(`Ошибка при ${attemptedAction} или сохранении конфигурации: ${e}`)
    }
  }

  const handleDownloadBinaries = async () => {
    try {
      await tauri.downloadBinaries()
    }
    catch (e) {
      console.error(e)
      reset()
      toast.error(`Ошибка загрузки файлов: ${e}`)
    }
  }

  if (initError) {
    return (
      <div className="flex h-full items-center justify-center p-8">
        <Alert variant="destructive" className="max-w-lg">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>Ошибка инициализации</AlertTitle>
          <AlertDescription>{initError}</AlertDescription>
        </Alert>
      </div>
    )
  }

  if (!initialized) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="h-6 w-6 animate-spin" />
      </div>
    )
  }

  if (!isElevated) {
    return (
      <div className="flex h-full items-center justify-center p-8">
        <Alert variant="destructive" className="max-w-lg">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>Требуются права администратора</AlertTitle>
          <AlertDescription>
            Для работы WinDivert необходимы права администратора. Запустите
            приложение от имени администратора.
          </AlertDescription>
        </Alert>
      </div>
    )
  }

  if (binariesOk === false) {
    return (
      <div className="flex h-full items-center justify-center p-8">
        <Card className="max-w-md space-y-4 p-6 text-center">
          <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-accent">
            <AlertCircle className="h-8 w-8 text-muted-foreground" />
          </div>
          <div>
            <h2 className="text-lg font-semibold">Требуется обновление файлов</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Бинарные файлы, fake-пакеты или фильтры отсутствуют либо повреждены. Обновление выполняется только вручную.
            </p>
          </div>
          {isDownloading && progress
            ? (
                <div className="space-y-2">
                  <div className="h-2 overflow-hidden rounded-full bg-muted">
                    <div
                      className="h-full bg-primary transition-all duration-300"
                      style={{
                        width: `${Math.max(0, Math.min(100, progress.total > 0 ? (progress.current / progress.total) * 100 : 0))}%`,
                      }}
                    />
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {progress.current}
                    /
                    {progress.total}
                    :
                    {progress.filename}
                  </p>
                </div>
              )
            : (
                <Button onClick={handleDownloadBinaries} className="w-full">
                  Обновить файлы
                </Button>
              )}
        </Card>
      </div>
    )
  }

  return (
    <div className="relative flex h-full flex-col">
      <div className="relative z-10 flex flex-1 items-center justify-center p-8">
        <div className="space-y-6 text-center">
          <div className="relative">
            <Button
              onClick={handleToggleConnection}
              disabled={
                status === 'connecting' || status === 'disconnecting' || !config
              }
              variant="ghost"
              className={cn(
                'h-32 w-32 rounded-full transition-all duration-300',
                status === 'connected'
                && 'animate-pulse-glow bg-green-600 text-white hover:bg-green-500 dark:hover:bg-green-500',
                status === 'connecting'
                && 'bg-yellow-600 text-white hover:bg-yellow-500',
                status === 'disconnecting'
                && 'bg-orange-600 text-white hover:bg-orange-500',
                status === 'error' && 'bg-red-600 text-white hover:bg-red-500',
                status === 'disconnected'
                && 'animate-pulse-glow-primary bg-neutral-900 text-white hover:bg-neutral-800 dark:bg-neutral-100 dark:text-neutral-900 dark:hover:bg-neutral-200',
              )}
            >
              <Power className="size-12" />
            </Button>
            {(status === 'connecting' || status === 'disconnecting') && (
              <div className="absolute inset-0 rounded-full bg-yellow-500/30 animate-ping" />
            )}
          </div>

          <div>
            <h2 className="text-2xl font-semibold">
              {status === 'connected'
                ? 'Подключено'
                : status === 'connecting'
                  ? 'Подключение...'
                  : status === 'disconnecting'
                    ? 'Отключение...'
                    : status === 'error'
                      ? 'Ошибка'
                      : 'Отключено'}
            </h2>
          </div>

          <ToggleGroup
            type="single"
            value={config?.listMode ?? 'ipset'}
            onValueChange={handleListModeChange}
            className="justify-center gap-1"
            disabled={status !== 'disconnected'}
          >
            {status === 'disconnected'
              ? (
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <div>
                        <ToggleGroupItem
                          value="ipset"
                          className="px-3 py-1.5 text-xs data-[state=on]:bg-green-500/20 data-[state=on]:text-green-600 dark:data-[state=on]:text-green-400 disabled:cursor-not-allowed disabled:opacity-50"
                        >
                          Только заблокированные
                        </ToggleGroupItem>
                      </div>
                    </TooltipTrigger>
                    <TooltipContent className="max-w-xs text-center">
                      Обрабатываются только заблокированные в России IP-адреса. Достоверность 99.9%
                    </TooltipContent>
                  </Tooltip>
                )
              : (
                  <ToggleGroupItem
                    value="ipset"
                    className="px-3 py-1.5 text-xs data-[state=on]:bg-green-500/20 data-[state=on]:text-green-600 dark:data-[state=on]:text-green-400 disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    Только заблокированные
                  </ToggleGroupItem>
                )}
            {status === 'disconnected'
              ? (
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <div>
                        <ToggleGroupItem
                          value="exclude"
                          className="px-3 py-1.5 text-xs data-[state=on]:bg-amber-500/20 data-[state=on]:text-amber-600 dark:data-[state=on]:text-amber-400 disabled:cursor-not-allowed disabled:opacity-50"
                        >
                          Исключения
                        </ToggleGroupItem>
                      </div>
                    </TooltipTrigger>
                    <TooltipContent className="max-w-xs text-center">
                      По умолчанию обрабатываются все адреса, кроме тех, которые стратегии ломают
                    </TooltipContent>
                  </Tooltip>
                )
              : (
                  <ToggleGroupItem
                    value="exclude"
                    className="px-3 py-1.5 text-xs data-[state=on]:bg-amber-500/20 data-[state=on]:text-amber-600 dark:data-[state=on]:text-amber-400 disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    Исключения
                  </ToggleGroupItem>
                )}
          </ToggleGroup>
        </div>
      </div>
    </div>
  )
}
