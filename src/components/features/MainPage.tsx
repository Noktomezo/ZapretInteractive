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
import Waves from '@/components/Waves'
import * as tauri from '@/lib/tauri'
import { cn } from '@/lib/utils'
import { useAppStore } from '@/stores/app.store'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'
import { useDownloadStore } from '@/stores/download.store'

export function MainPage() {
  const availableUpdatesPromptKeyRef = useRef('')
  const [initError, setInitError] = useState<string | null>(null)
  const { config, setListMode } = useConfigStore()
  const { status, connect, disconnect } = useConnectionStore()
  const { isDownloading, progress, reset }
    = useDownloadStore()
  const { initialized, isElevated, binariesOk, missingCriticalFiles, availableUpdates, configMissing, initialize, setConfigMissing }
    = useAppStore()

  const waveColor = status === 'connected'
    ? 'rgba(74, 222, 128, 0.18)'
    : status === 'connecting' || status === 'disconnecting'
      ? 'rgba(250, 204, 21, 0.16)'
      : 'rgba(248, 113, 113, 0.14)'

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
    const { status: currentStatus } = useConnectionStore.getState()
    const shouldReconnect = currentStatus === 'connected'
    try {
      if (currentStatus === 'connected' || currentStatus === 'connecting' || currentStatus === 'disconnecting') {
        await disconnect()
      }
      await tauri.downloadBinaries()

      if (shouldReconnect) {
        await connect()
      }
    }
    catch (e) {
      console.error(e)
      reset()
      toast.error(`Ошибка загрузки файлов: ${e}`)
    }
  }

  const handleRestoreDefaultConfig = async () => {
    try {
      await useConfigStore.getState().reset()
      await tauri.restoreDefaultFilters()
      setConfigMissing(false)
      toast.success('Конфигурация по умолчанию восстановлена')
    }
    catch (e) {
      toast.error(`Ошибка восстановления конфигурации: ${e}`)
    }
  }

  const handleRebuildHashes = async () => {
    try {
      await tauri.restoreHashesFromDisk()
      const binaries = await tauri.verifyBinaries()
      const files = await tauri.getMissingCriticalFiles()
      const updates = binaries ? await tauri.getAvailableUpdates() : []
      useAppStore.getState().setBinariesOk(binaries)
      useAppStore.getState().setMissingCriticalFiles(files)
      useAppStore.getState().setAvailableUpdates(updates)
    }
    catch (e) {
      toast.error(`Ошибка восстановления hashes.json: ${e}`)
    }
  }

  useEffect(() => {
    if (!initialized || binariesOk !== true || availableUpdates.length === 0)
      return

    const promptKey = availableUpdates.join('|')
    if (availableUpdatesPromptKeyRef.current === promptKey)
      return

    availableUpdatesPromptKeyRef.current = promptKey
    toast('Доступны обновления файлов', {
      description: availableUpdates.length === 1
        ? availableUpdates[0]
        : `${availableUpdates.length} файлов: ${availableUpdates.slice(0, 4).join(', ')}${availableUpdates.length > 4 ? '…' : ''}`,
      action: {
        label: 'Обновить',
        onClick: () => {
          void handleDownloadBinaries()
        },
      },
    })
  }, [availableUpdates, binariesOk, initialized])

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

  if (isDownloading) {
    return (
      <div className="flex h-full items-center justify-center p-8">
        <Card className="max-w-md space-y-4 p-6 text-center">
          <div>
            <h2 className="text-lg font-medium">Обновление файлов</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Интерфейс временно заблокирован до завершения обновления.
            </p>
          </div>
          {progress
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
                <Button disabled className="w-full">
                  Обновление...
                </Button>
              )}
        </Card>
      </div>
    )
  }

  if (configMissing) {
    return (
      <div className="flex h-full items-center justify-center p-8">
        <Card className="max-w-md space-y-4 p-6 text-center">
          <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-accent">
            <AlertCircle className="h-8 w-8 text-muted-foreground" />
          </div>
          <div>
            <h2 className="text-lg font-medium">Отсутствует конфигурация</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Файл `config.json` был удалён или недоступен. Можно восстановить конфигурацию по умолчанию.
            </p>
          </div>
          <Button onClick={handleRestoreDefaultConfig} className="w-full">
            Использовать дефолтный конфиг
          </Button>
        </Card>
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
            <h2 className="text-lg font-medium">Требуется обновление файлов</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Один или несколько критичных файлов приложения отсутствуют либо повреждены. Загрузятся только нужные файлы.
            </p>
            {missingCriticalFiles.length > 0 && (
              <p className="mt-2 text-xs text-muted-foreground">
                {missingCriticalFiles.length === 1
                  ? `Требует восстановления: ${missingCriticalFiles[0]}`
                  : `Требуют восстановления: ${missingCriticalFiles.slice(0, 4).join(', ')}${missingCriticalFiles.length > 4 ? '…' : ''}`}
              </p>
            )}
          </div>
          <Button onClick={handleDownloadBinaries} className="w-full">
            Загрузить нужные файлы
          </Button>
          {missingCriticalFiles.length === 0 && (
            <Button variant="outline" onClick={handleRebuildHashes} className="w-full">
              Восстановить hashes.json
            </Button>
          )}
        </Card>
      </div>
    )
  }

  return (
    <div className="relative flex h-full flex-col">
      <div className="pointer-events-none absolute inset-0 overflow-hidden">
        <Waves
          lineColor={waveColor}
          backgroundColor="transparent"
          waveSpeedX={0.008}
          waveSpeedY={0.004}
          waveAmpX={24}
          waveAmpY={12}
          xGap={14}
          yGap={26}
          friction={0.94}
          tension={0.004}
          maxCursorMove={80}
          className="opacity-90"
        />
      </div>
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
            <h2 className="text-2xl font-medium">
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
