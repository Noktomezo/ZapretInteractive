import type { ListMode } from '@/lib/types'
import { useNavigate } from '@tanstack/react-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import {
  AlertCircle,
  Power,
} from 'lucide-react'
import { useCallback, useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import FaultyTerminal from '@/components/FaultyTerminal'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
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
const LIST_MODE_OPTIONS: {
  value: ListMode
  label: string
  tooltip: string
  activeClassName: string
  indicatorClassName: string
}[] = [
  {
    value: 'ipset',
    label: 'Только заблокированные',
    tooltip: 'Обрабатываются только заблокированные в России IP-адреса. Достоверность 99.9%',
    activeClassName: 'data-[state=on]:text-success data-[state=on]:[text-shadow:0_0_12px_color-mix(in_oklab,var(--success)_32%,transparent)]',
    indicatorClassName: 'border-success/42 bg-success/20',
  },
  {
    value: 'exclude',
    label: 'Исключения',
    tooltip: 'По умолчанию обрабатываются все адреса, кроме тех, которые стратегии ломают',
    activeClassName: 'data-[state=on]:text-warning data-[state=on]:[text-shadow:0_0_12px_color-mix(in_oklab,var(--warning)_32%,transparent)]',
    indicatorClassName: 'border-warning/42 bg-warning/22',
  },
]

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
    <div className="pointer-events-none absolute top-0 right-0 bottom-0 w-screen overflow-hidden">
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
  const connectButtonRef = useRef<HTMLButtonElement | null>(null)
  const connectGlassAngleRef = useRef(0)
  const connectGlassSpeedRef = useRef(0.15)
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
  const activeListModeIndex = Math.max(LIST_MODE_OPTIONS.findIndex(option => option.value === selectedListMode), 0)
  const listModeOptionCount = LIST_MODE_OPTIONS.length
  const listModeIndicatorWidth = `calc((100% - 0.25rem - ${(listModeOptionCount - 1) * 0.125}rem) / ${listModeOptionCount})`
  const listModeDisabled = !initialized || !config || status !== 'disconnected' || listModeUpdating

  useEffect(() => {
    connectGlassSpeedRef.current = status === 'connecting' || status === 'disconnecting' ? 0.3 : 0.15
  }, [status])

  useEffect(() => {
    const reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)')

    let frameId: number | null = null
    let previousTimestamp = performance.now()

    const rotateConnectGlass = (timestamp: number) => {
      const elapsed = timestamp - previousTimestamp
      previousTimestamp = timestamp
      connectGlassAngleRef.current = (connectGlassAngleRef.current + elapsed * connectGlassSpeedRef.current) % 360
      connectButtonRef.current?.style.setProperty('--connect-glass-orbit-angle', `${connectGlassAngleRef.current}deg`)
      frameId = window.requestAnimationFrame(rotateConnectGlass)
    }

    const startConnectGlassRotation = () => {
      if (frameId !== null) {
        return
      }

      previousTimestamp = performance.now()
      frameId = window.requestAnimationFrame(rotateConnectGlass)
    }

    const stopConnectGlassRotation = () => {
      if (frameId === null) {
        return
      }

      window.cancelAnimationFrame(frameId)
      frameId = null
    }

    const handleReducedMotionChange = (event: MediaQueryListEvent) => {
      if (event.matches) {
        stopConnectGlassRotation()
        return
      }

      startConnectGlassRotation()
    }

    if (!reducedMotionQuery.matches) {
      startConnectGlassRotation()
    }

    const legacyReducedMotionQuery = reducedMotionQuery as MediaQueryList & {
      addListener?: (listener: (event: MediaQueryListEvent) => void) => void
      removeListener?: (listener: (event: MediaQueryListEvent) => void) => void
    }

    if (typeof reducedMotionQuery.addEventListener === 'function') {
      reducedMotionQuery.addEventListener('change', handleReducedMotionChange)
    }
    else {
      legacyReducedMotionQuery.addListener?.(handleReducedMotionChange)
    }

    return () => {
      stopConnectGlassRotation()

      if (typeof reducedMotionQuery.removeEventListener === 'function') {
        reducedMotionQuery.removeEventListener('change', handleReducedMotionChange)
      }
      else {
        legacyReducedMotionQuery.removeListener?.(handleReducedMotionChange)
      }
    }
  }, [])

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

  const renderListModeToggleItem = (option: typeof LIST_MODE_OPTIONS[number], key?: string) => (
    <ToggleGroupItem
      key={key}
      value={option.value}
      className={cn(
        'relative z-10 h-7.5 min-w-max cursor-pointer rounded-[calc(var(--radius)-0.125rem)] border-0 bg-transparent px-3 text-xs text-foreground/80 shadow-none transition-colors duration-300 hover:bg-transparent hover:text-foreground data-[state=on]:bg-transparent data-[state=on]:shadow-none',
        option.activeClassName,
        listModeDisabled && 'cursor-not-allowed opacity-50',
      )}
      aria-label={option.label}
    >
      {option.label}
    </ToggleGroupItem>
  )

  useMountEffect(() => {
    if (!useAppStore.getState().mainPageVisited) {
      setMainPageVisited(true)
    }
  })

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

  if (initialized && !isElevated) {
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

  if (initialized && configMissing) {
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

  if (initialized && binariesOk === false) {
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
      <svg className="pointer-events-none absolute size-0" aria-hidden="true" focusable="false">
        <filter id="connect-button-glass" x="0%" y="0%" width="100%" height="100%">
          <feTurbulence
            type="fractalNoise"
            baseFrequency="0.012 0.012"
            numOctaves="2"
            seed="92"
            result="noise"
          />
          <feGaussianBlur in="noise" stdDeviation="0.35" result="blur" />
          <feDisplacementMap
            in="SourceGraphic"
            in2="blur"
            scale="18"
            xChannelSelector="R"
            yChannelSelector="G"
          />
        </filter>
        <filter id="connect-icon-etched" x="-20%" y="-20%" width="140%" height="140%">
          <feTurbulence
            type="fractalNoise"
            baseFrequency="0.9"
            numOctaves="2"
            seed="31"
            result="noise"
          />
          <feDisplacementMap
            in="SourceGraphic"
            in2="noise"
            scale="0.9"
            xChannelSelector="R"
            yChannelSelector="G"
            result="etched"
          />
          <feGaussianBlur in="etched" stdDeviation="0.12" />
        </filter>
      </svg>
      <div className="space-y-6 text-center">
        <div className="relative">
          <Button
            ref={connectButtonRef}
            onClick={handleToggleConnection}
            disabled={
              !initialized || status === 'connecting' || status === 'disconnecting' || !config
            }
            variant="ghost"
            className={cn(
              'connect-liquid-glass h-32 w-32 rounded-full border border-white/10 bg-transparent shadow-lg shadow-black/10 backdrop-blur-xl transition-[background-color,color,box-shadow,transform,backdrop-filter] duration-500 ease-out hover:bg-transparent hover:text-current disabled:opacity-100 dark:hover:bg-transparent',
              status === 'connected'
              && 'connect-liquid-glass-success text-success-foreground dark:border-white/8 dark:text-success',
              status === 'connecting'
              && 'connect-liquid-glass-warning border-warning/22 text-warning-foreground dark:border-white/8 dark:text-warning',
              status === 'disconnecting'
              && 'connect-liquid-glass-warning border-warning/22 text-warning-foreground dark:border-white/8 dark:text-warning',
              status === 'error'
              && 'connect-liquid-glass-error text-destructive-foreground dark:border-white/8 dark:text-destructive',
              status === 'disconnected'
              && 'connect-liquid-glass-neutral text-foreground dark:border-white/8',
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
          value={selectedListMode}
          onValueChange={(value) => {
            if (value) {
              void handleListModeChange(value)
            }
          }}
          disabled={listModeDisabled}
          className="relative mx-auto grid w-fit gap-0.5 rounded-lg border border-border/60 bg-background/76 p-0.5 shadow-lg shadow-black/10 backdrop-blur-md"
          style={{ gridTemplateColumns: `repeat(${listModeOptionCount}, minmax(max-content, 1fr))` }}
          aria-label="Режим списков"
        >
          <div
            className={cn(
              'pointer-events-none absolute inset-y-0.5 left-0.5 rounded-[calc(var(--radius)-0.125rem)] border shadow-sm transition-all duration-300 ease-out',
              LIST_MODE_OPTIONS[activeListModeIndex].indicatorClassName,
            )}
            style={{
              width: listModeIndicatorWidth,
              transform: `translateX(calc(${activeListModeIndex} * (100% + 0.125rem)))`,
            }}
          />
          {LIST_MODE_OPTIONS.map(option => (
            status === 'disconnected'
              ? (
                  <Tooltip key={option.value}>
                    <TooltipTrigger asChild>
                      {renderListModeToggleItem(option)}
                    </TooltipTrigger>
                    <TooltipContent className="max-w-xs text-center">
                      {option.tooltip}
                    </TooltipContent>
                  </Tooltip>
                )
              : (
                  renderListModeToggleItem(option, option.value)
                )
          ))}
        </ToggleGroup>
      </div>
    </div>
  )
}
