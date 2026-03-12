import type { ListMode } from '@/lib/types'
import {
  AlertCircle,
  Loader2,
  Power,
} from 'lucide-react'
import { useCallback, useEffect, useRef, useState } from 'react'
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
import { runWithPausedConnection } from '@/lib/connection-flow'
import * as tauri from '@/lib/tauri'
import { cn } from '@/lib/utils'
import { useAppStore } from '@/stores/app.store'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'
import { useDownloadStore } from '@/stores/download.store'
import { useUpdaterStore } from '@/stores/updater.store'

export function MainPage() {
  const availableUpdatesPromptKeyRef = useRef('')
  const dismissedPromptKeysRef = useRef<Set<string>>(new Set())
  const activeUpdateToastIdRef = useRef<string | number | null>(null)
  const appUpdatePromptKeyRef = useRef('')
  const activeAppUpdateToastIdRef = useRef<string | number | null>(null)
  const [initError, setInitError] = useState<string | null>(null)
  const [listModeUpdating, setListModeUpdating] = useState(false)
  const config = useConfigStore(state => state.config)
  const applyPersistedListMode = useConfigStore(state => state.applyPersistedListMode)
  const saveNow = useConfigStore(state => state.saveNow)
  const setCoreFileUpdatePromptsEnabled = useConfigStore(state => state.setCoreFileUpdatePromptsEnabled)
  const setAppAutoUpdatesEnabled = useConfigStore(state => state.setAppAutoUpdatesEnabled)
  const status = useConnectionStore(state => state.status)
  const connect = useConnectionStore(state => state.connect)
  const disconnect = useConnectionStore(state => state.disconnect)
  const isDownloading = useDownloadStore(state => state.isDownloading)
  const progress = useDownloadStore(state => state.progress)
  const resetDownload = useDownloadStore(state => state.reset)
  const initialized = useAppStore(state => state.initialized)
  const isElevated = useAppStore(state => state.isElevated)
  const binariesOk = useAppStore(state => state.binariesOk)
  const missingCriticalFiles = useAppStore(state => state.missingCriticalFiles)
  const availableUpdates = useAppStore(state => state.availableUpdates)
  const configMissing = useAppStore(state => state.configMissing)
  const initialize = useAppStore(state => state.initialize)
  const setConfigMissing = useAppStore(state => state.setConfigMissing)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)
  const appUpdate = useUpdaterStore(state => state.availableUpdate)
  const appUpdateDownloading = useUpdaterStore(state => state.downloading)
  const appUpdateInstalling = useUpdaterStore(state => state.installing)
  const dismissedAppUpdateVersion = useUpdaterStore(state => state.dismissedVersionThisSession)
  const installAvailableAppUpdate = useUpdaterStore(state => state.installAvailableUpdate)
  const dismissCurrentAppUpdate = useUpdaterStore(state => state.dismissCurrentVersionUntilRestart)

  const waveColor = status === 'connected'
    ? 'rgba(74, 222, 128, 0.18)'
    : status === 'connecting' || status === 'disconnecting'
      ? 'rgba(250, 204, 21, 0.16)'
      : 'rgba(248, 113, 113, 0.35)'

  const handleListModeChange = async (value: string) => {
    if (!value || !config || listModeUpdating || value === config.listMode) {
      return
    }

    setListModeUpdating(true)
    try {
      await tauri.updateListMode(value as ListMode)
      addConfigLog(
        value === 'ipset'
          ? 'режим списков переключён на "Только заблокированные"'
          : 'режим списков переключён на "Исключения"',
      )
    }
    catch (e) {
      toast.error(`Не удалось переключить режим списков: ${e instanceof Error ? e.message : String(e)}`)
    }
    finally {
      setListModeUpdating(false)
    }
  }

  useEffect(() => {
    initialize().catch((error) => {
      console.error('Failed to initialize app:', error)
      setInitError(String(error))
    })

    const unlistenListMode = tauri.onListModeChanged((mode) => {
      applyPersistedListMode(mode)
    })

    return () => {
      unlistenListMode()
    }
  }, [applyPersistedListMode, initialize])

  const handleToggleConnection = async () => {
    const attemptedAction = status === 'connected' ? 'отключение' : 'подключение'
    try {
      if (status === 'connected') {
        await disconnect()
      }
      else {
        await connect()
      }
      await saveNow()
    }
    catch (e) {
      toast.error(`Ошибка при ${attemptedAction} или сохранении конфигурации: ${e}`)
    }
  }

  const handleDownloadBinaries = useCallback(async () => {
    try {
      await runWithPausedConnection(async () => {
        await tauri.downloadBinaries()
      })
    }
    catch (e) {
      console.error(e)
      resetDownload()
      toast.error(`Ошибка загрузки файлов: ${e}`)
    }
  }, [resetDownload])

  const dismissActiveUpdateToast = useCallback(() => {
    if (activeUpdateToastIdRef.current !== null) {
      toast.dismiss(activeUpdateToastIdRef.current)
      activeUpdateToastIdRef.current = null
    }
  }, [])

  const dismissActiveAppUpdateToast = useCallback(() => {
    if (activeAppUpdateToastIdRef.current !== null) {
      toast.dismiss(activeAppUpdateToastIdRef.current)
      activeAppUpdateToastIdRef.current = null
    }
  }, [])

  const handleApplyCoreFileUpdates = useCallback(async () => {
    try {
      await runWithPausedConnection(async () => {
        await tauri.applyCoreFileUpdates()
      })
      dismissActiveUpdateToast()
    }
    catch (e) {
      console.error(e)
      resetDownload()
      toast.error(`Ошибка обновления файлов: ${e}`)
    }
  }, [dismissActiveUpdateToast, resetDownload])

  const handleDisableCoreFileUpdatePrompts = useCallback(async () => {
    if (!config)
      return

    const previous = config.coreFileUpdatePromptsEnabled ?? true
    setCoreFileUpdatePromptsEnabled(false)
    try {
      await saveNow()
      addConfigLog('автопредложения обновления winws/fake файлов отключены')
      dismissActiveUpdateToast()
    }
    catch (e) {
      setCoreFileUpdatePromptsEnabled(previous)
      toast.error(`Не удалось отключить предложения обновления: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [addConfigLog, config, dismissActiveUpdateToast, saveNow, setCoreFileUpdatePromptsEnabled])

  const handleInstallAppUpdate = useCallback(async () => {
    try {
      await installAvailableAppUpdate()
      dismissActiveAppUpdateToast()
    }
    catch (e) {
      console.error(e)
    }
  }, [dismissActiveAppUpdateToast, installAvailableAppUpdate])

  const handleDisableAppAutoUpdates = useCallback(async () => {
    if (!config) {
      return
    }

    const previous = config.appAutoUpdatesEnabled ?? true
    setAppAutoUpdatesEnabled(false)
    try {
      await saveNow()
      addConfigLog('автоматическая проверка обновлений приложения отключена')
      dismissActiveAppUpdateToast()
    }
    catch (e) {
      setAppAutoUpdatesEnabled(previous)
      toast.error(`Не удалось отключить автообновления приложения: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [addConfigLog, config, dismissActiveAppUpdateToast, saveNow, setAppAutoUpdatesEnabled])

  const handleRestoreDefaultConfig = async () => {
    try {
      await useConfigStore.getState().reset()
      await tauri.ensureManagedFiles()
      setConfigMissing(false)
      addConfigLog('восстановлена конфигурация по умолчанию')
      toast.success('Конфигурация по умолчанию восстановлена')
    }
    catch (e) {
      toast.error(`Ошибка восстановления конфигурации: ${e}`)
    }
  }

  const handleRebuildHashes = async () => {
    try {
      await tauri.restoreHashesFromDisk()
      await useAppStore.getState().refreshRemoteState()
      toast.success('hashes.json восстановлен')
    }
    catch (e) {
      toast.error(`Ошибка восстановления hashes.json: ${e}`)
    }
  }

  useEffect(() => {
    const promptEnabled = config?.coreFileUpdatePromptsEnabled ?? true

    if (!initialized || binariesOk !== true || availableUpdates.length === 0 || !promptEnabled) {
      dismissActiveUpdateToast()
      return
    }

    const promptKey = availableUpdates.join('|')
    if (dismissedPromptKeysRef.current.has(promptKey))
      return

    if (availableUpdatesPromptKeyRef.current === promptKey && activeUpdateToastIdRef.current !== null)
      return

    dismissActiveUpdateToast()
    availableUpdatesPromptKeyRef.current = promptKey
    activeUpdateToastIdRef.current = toast.custom(() => (
      <div className="w-[420px] rounded-lg border bg-background p-4 shadow-lg">
        <div className="space-y-1">
          <p className="text-sm font-medium">Доступны обновления файлов</p>
          <p className="text-xs text-muted-foreground">
            {availableUpdates.length === 1
              ? availableUpdates[0]
              : `${availableUpdates.length} файлов: ${availableUpdates.slice(0, 4).join(', ')}${availableUpdates.length > 4 ? '…' : ''}`}
          </p>
        </div>
        <div className="mt-3 flex gap-2">
          <Button size="sm" onClick={() => { void handleApplyCoreFileUpdates() }}>
            Обновить
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              dismissedPromptKeysRef.current.add(promptKey)
              dismissActiveUpdateToast()
            }}
          >
            Не сейчас
          </Button>
          <Button size="sm" variant="ghost" onClick={() => { void handleDisableCoreFileUpdatePrompts() }}>
            Не предлагать
          </Button>
        </div>
      </div>
    ), { duration: Number.POSITIVE_INFINITY })
  }, [availableUpdates, binariesOk, config?.coreFileUpdatePromptsEnabled, dismissActiveUpdateToast, handleApplyCoreFileUpdates, handleDisableCoreFileUpdatePrompts, initialized])

  useEffect(() => {
    if (availableUpdates.length === 0) {
      availableUpdatesPromptKeyRef.current = ''
      dismissActiveUpdateToast()
    }
  }, [availableUpdates.length, dismissActiveUpdateToast])

  useEffect(() => {
    const appUpdatesEnabled = config?.appAutoUpdatesEnabled ?? true

    if (
      !initialized
      || !appUpdate
      || !appUpdatesEnabled
      || dismissedAppUpdateVersion === appUpdate.version
      || appUpdateDownloading
      || appUpdateInstalling
    ) {
      dismissActiveAppUpdateToast()
      return
    }

    const promptKey = `app-update:${appUpdate.version}`
    if (appUpdatePromptKeyRef.current === promptKey && activeAppUpdateToastIdRef.current !== null) {
      return
    }

    const notesPreview = appUpdate.notes?.split('\n').find(line => line.trim().length > 0)?.trim()

    dismissActiveAppUpdateToast()
    appUpdatePromptKeyRef.current = promptKey
    activeAppUpdateToastIdRef.current = toast.custom(() => (
      <div className="w-[420px] rounded-lg border bg-background p-4 shadow-lg">
        <div className="space-y-1">
          <p className="text-sm font-medium">Доступна новая версия приложения</p>
          <p className="text-xs text-muted-foreground">
            Доступна версия
            {' '}
            {appUpdate.version}
          </p>
          {notesPreview && (
            <p className="text-xs text-muted-foreground line-clamp-2">{notesPreview}</p>
          )}
        </div>
        <div className="mt-3 flex gap-2">
          <Button size="sm" onClick={() => { void handleInstallAppUpdate() }}>
            Да
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              dismissCurrentAppUpdate()
              dismissActiveAppUpdateToast()
            }}
          >
            Нет
          </Button>
          <Button size="sm" variant="ghost" onClick={() => { void handleDisableAppAutoUpdates() }}>
            Отключить автообновления
          </Button>
        </div>
      </div>
    ), { duration: Number.POSITIVE_INFINITY })
  }, [
    appUpdate,
    appUpdateDownloading,
    appUpdateInstalling,
    config?.appAutoUpdatesEnabled,
    dismissActiveAppUpdateToast,
    dismissCurrentAppUpdate,
    dismissedAppUpdateVersion,
    handleDisableAppAutoUpdates,
    handleInstallAppUpdate,
    initialized,
  ])

  useEffect(() => {
    if (!appUpdate) {
      appUpdatePromptKeyRef.current = ''
      dismissActiveAppUpdateToast()
    }
  }, [appUpdate, dismissActiveAppUpdateToast])

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
            <h2 className="text-lg font-medium">Не удалось автоматически восстановить конфигурацию</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Файл
              {' '}
              <code>config.json</code>
              {' '}
              недоступен или не был восстановлен автоматически. Можно повторить восстановление дефолтного конфига вручную.
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
                && 'animate-pulse-glow-yellow bg-yellow-600 text-white hover:bg-yellow-500',
                status === 'disconnecting'
                && 'animate-pulse-glow-yellow bg-orange-600 text-white hover:bg-orange-500',
                status === 'error' && 'bg-red-600 text-white hover:bg-red-500',
                status === 'disconnected'
                && 'animate-pulse-glow-primary bg-neutral-900 text-white hover:bg-neutral-800 dark:bg-neutral-100 dark:text-neutral-900 dark:hover:bg-neutral-200',
              )}
            >
              <Power className="size-12" />
            </Button>
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
            disabled={status !== 'disconnected' || listModeUpdating}
          >
            {status === 'disconnected'
              ? (
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <div>
                        <ToggleGroupItem
                          value="ipset"
                          disabled={listModeUpdating}
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
                    disabled={listModeUpdating}
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
                          disabled={listModeUpdating}
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
                    disabled={listModeUpdating}
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
