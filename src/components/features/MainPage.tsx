import type { ListMode } from '@/lib/types'
import { useNavigate } from '@tanstack/react-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import {
  AlertCircle,
  Loader2,
  Power,
} from 'lucide-react'
import { useCallback, useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import FaultyTerminal from '@/components/FaultyTerminal'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { runWithPausedConnection } from '@/lib/connection-flow'
import * as tauri from '@/lib/tauri'
import { cn } from '@/lib/utils'
import { useAppStore } from '@/stores/app.store'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'
import { useDownloadStore } from '@/stores/download.store'
import { useUpdaterStore } from '@/stores/updater.store'

const terminalGridMul: [number, number] = [2, 1]

export function MainPageTerminalBackdrop({ visible }: { visible: boolean }) {
  const status = useConnectionStore(state => state.status)
  const mainTerminalTimeOffset = useAppStore(state => state.mainTerminalTimeOffset)
  const mainPageVisited = useAppStore(state => state.mainPageVisited)
  const terminalTint = status === 'connected'
    ? 'var(--terminal-tint-success)'
    : status === 'connecting' || status === 'disconnecting'
      ? 'var(--terminal-tint-warning)'
      : 'var(--terminal-tint-danger)'
  const terminalBackgroundTint = 'var(--terminal-background-tint)'
  const terminalFlickerAmount = status === 'connected' ? 0 : 1
  const terminalCurvature = status === 'connected' || status === 'disconnecting' ? 0 : 0.1
  const terminalScanlineIntensity = status === 'disconnected' ? 0.22 : 0
  const [isTerminalVisible, setIsTerminalVisible] = useState(true)

  useMountEffect(() => {
    const appWindow = getCurrentWindow()
    let disposed = false

    const syncTerminalVisibility = async () => {
      const pageVisible = typeof document === 'undefined' || document.visibilityState === 'visible'

      try {
        const windowVisible = await appWindow.isVisible()
        if (!disposed) {
          setIsTerminalVisible(pageVisible && windowVisible)
        }
      }
      catch {
        if (!disposed) {
          setIsTerminalVisible(pageVisible)
        }
      }
    }

    const handleVisibilityChange = () => {
      void syncTerminalVisibility()
    }

    document.addEventListener('visibilitychange', handleVisibilityChange)
    window.addEventListener('focus', handleVisibilityChange)
    window.addEventListener('blur', handleVisibilityChange)

    const visibilityPollId = window.setInterval(() => {
      void syncTerminalVisibility()
    }, 1000)

    const unlistenFocusChangedPromise = appWindow.onFocusChanged(() => {
      void syncTerminalVisibility()
    })

    void syncTerminalVisibility()

    return () => {
      disposed = true
      document.removeEventListener('visibilitychange', handleVisibilityChange)
      window.removeEventListener('focus', handleVisibilityChange)
      window.removeEventListener('blur', handleVisibilityChange)
      window.clearInterval(visibilityPollId)
      void unlistenFocusChangedPromise.then(unlisten => unlisten())
    }
  })

  return (
    <div className="absolute inset-0 overflow-hidden">
      <FaultyTerminal
        aria-hidden="true"
        scale={1.5}
        gridMul={terminalGridMul}
        digitSize={1.2}
        timeScale={0.1}
        timeOffset={mainTerminalTimeOffset}
        pause={!visible || !isTerminalVisible}
        scanlineIntensity={terminalScanlineIntensity}
        glitchAmount={1}
        flickerAmount={terminalFlickerAmount}
        noiseAmp={1}
        chromaticAberration={0}
        dither={0}
        curvature={terminalCurvature}
        tint={terminalTint}
        backgroundTint={terminalBackgroundTint}
        mouseReact
        mouseStrength={0.5}
        pageLoadAnimation={visible && !mainPageVisited}
        brightness={0.55}
        className={cn(
          'pointer-events-none transition-opacity duration-200',
          visible ? 'opacity-90' : 'opacity-0',
        )}
        role="presentation"
        tabIndex={-1}
      />
    </div>
  )
}

export function MainPage() {
  const navigate = useNavigate()
  const availableUpdatesPromptKeyRef = useRef('')
  const dismissedPromptKeysRef = useRef<Set<string>>(new Set())
  const activeUpdateToastIdRef = useRef<string | number | null>(null)
  const appUpdatePromptKeyRef = useRef('')
  const activeAppUpdateToastIdRef = useRef<string | number | null>(null)
  const listModeButtonRefs = useRef<Array<HTMLButtonElement | null>>([])
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
  const setMainPageVisited = useAppStore(state => state.setMainPageVisited)
  const setConfigMissing = useAppStore(state => state.setConfigMissing)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)
  const appUpdate = useUpdaterStore(state => state.availableUpdate)
  const appUpdateDownloading = useUpdaterStore(state => state.downloading)
  const appUpdateInstalling = useUpdaterStore(state => state.installing)
  const dismissedAppUpdateVersion = useUpdaterStore(state => state.dismissedVersionThisSession)
  const installAvailableAppUpdate = useUpdaterStore(state => state.installAvailableUpdate)
  const dismissCurrentAppUpdate = useUpdaterStore(state => state.dismissCurrentVersionUntilRestart)
  const selectedListMode = config?.listMode ?? 'ipset'
  const [focusedListModeIndex, setFocusedListModeIndex] = useState<number | null>(null)
  const activeListModeIndex = focusedListModeIndex ?? (selectedListMode === 'exclude' ? 1 : 0)
  const listModeDisabled = !config || status !== 'disconnected' || listModeUpdating

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

  useMountEffect(() => {
    if (!useAppStore.getState().mainPageVisited) {
      setMainPageVisited(true)
    }
  })

  const handleListModeKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>, index: number) => {
    if (listModeDisabled) {
      return
    }

    const modes: ListMode[] = ['ipset', 'exclude']
    let nextIndex: number | null = null

    switch (event.key) {
      case 'ArrowLeft':
      case 'ArrowUp':
        nextIndex = (index + modes.length - 1) % modes.length
        break
      case 'ArrowRight':
      case 'ArrowDown':
        nextIndex = (index + 1) % modes.length
        break
      case 'Home':
        nextIndex = 0
        break
      case 'End':
        nextIndex = modes.length - 1
        break
      default:
        return
    }

    event.preventDefault()
    setFocusedListModeIndex(nextIndex)
    listModeButtonRefs.current[nextIndex]?.focus()
    void handleListModeChange(modes[nextIndex])
  }

  useMountEffect(() => {
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
  })

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
      <div className="w-full min-w-0">
        <div className="space-y-1">
          <p className="text-sm font-medium">Доступны обновления файлов</p>
          <p className="text-xs text-muted-foreground">
            {availableUpdates.length === 1
              ? availableUpdates[0]
              : `${availableUpdates.length} файлов: ${availableUpdates.slice(0, 4).join(', ')}${availableUpdates.length > 4 ? '…' : ''}`}
          </p>
        </div>
        <div className="mt-3 flex flex-wrap gap-2">
          <Button
            size="sm"
            onClick={() => {
              void (async () => {
                await navigate({ to: '/' })
                await handleApplyCoreFileUpdates()
              })()
            }}
          >
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

    dismissActiveAppUpdateToast()
    appUpdatePromptKeyRef.current = promptKey
    activeAppUpdateToastIdRef.current = toast.custom(() => (
      <div className="w-full min-w-0">
        <div className="space-y-1">
          <p className="text-sm font-medium">Доступна новая версия приложения</p>
          <p className="text-xs text-muted-foreground">
            Доступна версия
            {' '}
            {appUpdate.version}
          </p>
        </div>
        <div className="mt-3 flex flex-wrap gap-2">
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
    <div className="relative z-10 flex h-full flex-1 items-center justify-center p-8">
      <div className="space-y-6 text-center">
        <div className="relative">
          <Button
            onClick={handleToggleConnection}
            disabled={
              status === 'connecting' || status === 'disconnecting' || !config
            }
            variant="ghost"
            className={cn(
              'h-32 w-32 rounded-full border border-white/10 shadow-lg shadow-black/10 backdrop-blur-xl transition-[background-color,color,box-shadow,transform,backdrop-filter] duration-500 ease-out disabled:opacity-100',
              status === 'connected'
              && 'animate-pulse-glow bg-success/48 text-white hover:border-white/14 hover:bg-success/60 hover:backdrop-blur-xl dark:border-white/8 dark:text-background',
              status === 'connecting'
              && 'animate-pulse-glow-yellow border-warning/30 bg-warning/48 text-white dark:border-white/8 dark:text-background',
              status === 'disconnecting'
              && 'animate-pulse-glow-yellow border-warning/30 bg-warning/48 text-white dark:border-white/8 dark:text-background',
              status === 'error' && 'bg-destructive/48 text-white hover:border-white/14 hover:bg-destructive/60 hover:backdrop-blur-xl dark:border-white/8 dark:text-background',
              status === 'disconnected'
              && 'animate-pulse-glow-neutral bg-foreground/48 text-background hover:border-white/14 hover:bg-foreground/60 hover:backdrop-blur-xl dark:border-white/8',
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

        <div
          role="radiogroup"
          aria-label="Режим списков"
          className="relative mx-auto grid w-fit grid-cols-2 gap-1 rounded-xl border border-border/60 bg-background/76 p-1 shadow-lg shadow-black/10 backdrop-blur-md"
        >
          <div
            className={cn(
              'pointer-events-none absolute inset-y-1 left-1 w-[calc(50%-0.25rem)] rounded-md border shadow-sm transition-all duration-300 ease-out',
              selectedListMode === 'ipset'
                ? 'translate-x-0 border-success/30 bg-success/10'
                : 'translate-x-full border-warning/30 bg-warning/12',
            )}
          />
          {status === 'disconnected'
            ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      ref={(element) => {
                        listModeButtonRefs.current[0] = element
                      }}
                      type="button"
                      role="radio"
                      aria-checked={selectedListMode === 'ipset'}
                      disabled={listModeDisabled}
                      tabIndex={activeListModeIndex === 0 ? 0 : -1}
                      onFocus={() => setFocusedListModeIndex(0)}
                      onKeyDown={event => handleListModeKeyDown(event, 0)}
                      onClick={() => void handleListModeChange('ipset')}
                      className={cn(
                        'relative z-10 h-8 cursor-pointer rounded-md px-3 text-xs font-medium transition-colors duration-300',
                        selectedListMode === 'ipset'
                          ? 'text-success'
                          : 'text-foreground/80 hover:text-foreground',
                        listModeDisabled && 'cursor-not-allowed opacity-50',
                      )}
                    >
                      Только заблокированные
                    </button>
                  </TooltipTrigger>
                  <TooltipContent className="max-w-xs text-center">
                    Обрабатываются только заблокированные в России IP-адреса. Достоверность 99.9%
                  </TooltipContent>
                </Tooltip>
              )
            : (
                <button
                  ref={(element) => {
                    listModeButtonRefs.current[0] = element
                  }}
                  type="button"
                  role="radio"
                  aria-checked={selectedListMode === 'ipset'}
                  disabled={listModeDisabled}
                  tabIndex={activeListModeIndex === 0 ? 0 : -1}
                  onFocus={() => setFocusedListModeIndex(0)}
                  onKeyDown={event => handleListModeKeyDown(event, 0)}
                  onClick={() => void handleListModeChange('ipset')}
                  className={cn(
                    'relative z-10 h-8 cursor-pointer rounded-md px-3 text-xs font-medium transition-colors duration-300',
                    selectedListMode === 'ipset'
                      ? 'text-success'
                      : 'text-foreground/80 hover:text-foreground',
                    listModeDisabled && 'cursor-not-allowed opacity-50',
                  )}
                >
                  Только заблокированные
                </button>
              )}
          {status === 'disconnected'
            ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      ref={(element) => {
                        listModeButtonRefs.current[1] = element
                      }}
                      type="button"
                      role="radio"
                      aria-checked={selectedListMode === 'exclude'}
                      disabled={listModeDisabled}
                      tabIndex={activeListModeIndex === 1 ? 0 : -1}
                      onFocus={() => setFocusedListModeIndex(1)}
                      onKeyDown={event => handleListModeKeyDown(event, 1)}
                      onClick={() => void handleListModeChange('exclude')}
                      className={cn(
                        'relative z-10 h-8 cursor-pointer rounded-md px-3 text-xs font-medium transition-colors duration-300',
                        selectedListMode === 'exclude'
                          ? 'text-warning'
                          : 'text-foreground/80 hover:text-foreground',
                        listModeDisabled && 'cursor-not-allowed opacity-50',
                      )}
                    >
                      Исключения
                    </button>
                  </TooltipTrigger>
                  <TooltipContent className="max-w-xs text-center">
                    По умолчанию обрабатываются все адреса, кроме тех, которые стратегии ломают
                  </TooltipContent>
                </Tooltip>
              )
            : (
                <button
                  ref={(element) => {
                    listModeButtonRefs.current[1] = element
                  }}
                  type="button"
                  role="radio"
                  aria-checked={selectedListMode === 'exclude'}
                  disabled={listModeDisabled}
                  tabIndex={activeListModeIndex === 1 ? 0 : -1}
                  onFocus={() => setFocusedListModeIndex(1)}
                  onKeyDown={event => handleListModeKeyDown(event, 1)}
                  onClick={() => void handleListModeChange('exclude')}
                  className={cn(
                    'relative z-10 h-8 cursor-pointer rounded-md px-3 text-xs font-medium transition-colors duration-300',
                    selectedListMode === 'exclude'
                      ? 'text-warning'
                      : 'text-foreground/80 hover:text-foreground',
                    listModeDisabled && 'cursor-not-allowed opacity-50',
                  )}
                >
                  Исключения
                </button>
              )}
        </div>
      </div>
    </div>
  )
}
