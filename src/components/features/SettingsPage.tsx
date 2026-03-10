import { Download, FolderOpen, Loader2, RotateCcw } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import { ThemeSwitcher } from '@/components/ThemeSwitcher'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Switch } from '@/components/ui/switch'
import * as tauri from '@/lib/tauri'
import { cn } from '@/lib/utils'
import { useAppStore } from '@/stores/app.store'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'
import { useDownloadStore } from '@/stores/download.store'

const RANGE_RE = /^\d+-\d+$/
const PORT_RE = /^\d+$/

function isValidPortRange(value: string): boolean {
  if (!value.trim())
    return true
  const parts = value.split(',').map(p => p.trim())
  for (const part of parts) {
    if (RANGE_RE.test(part)) {
      const [start, end] = part.split('-').map(p => Number.parseInt(p, 10))
      if (start < 1 || end > 65535 || start > end)
        return false
    }
    else if (PORT_RE.test(part)) {
      const port = Number.parseInt(part, 10)
      if (port < 1 || port > 65535)
        return false
    }
    else {
      return false
    }
  }
  return true
}

function waitForConnectionStatus(
  expectedStatus: 'connected' | 'disconnected',
  timeoutMs = 15000,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const currentStatus = useConnectionStore.getState().status
    if (currentStatus === expectedStatus) {
      resolve()
      return
    }
    if (currentStatus === 'error') {
      reject(new Error(`Connection entered error state while waiting for ${expectedStatus}`))
      return
    }

    let unsubscribe = () => {}
    const timeoutId = window.setTimeout(() => {
      unsubscribe()
      reject(new Error(`Timeout waiting for connection status: ${expectedStatus}`))
    }, timeoutMs)

    unsubscribe = useConnectionStore.subscribe((state) => {
      if (state.status === expectedStatus) {
        window.clearTimeout(timeoutId)
        unsubscribe()
        resolve()
      }
      else if (state.status === 'error') {
        window.clearTimeout(timeoutId)
        unsubscribe()
        reject(new Error(`Connection entered error state while waiting for ${expectedStatus}`))
      }
    })
  })
}

function waitForTerminalConnectionStatus(timeoutMs = 15000): Promise<'connected' | 'disconnected'> {
  return new Promise((resolve, reject) => {
    const currentStatus = useConnectionStore.getState().status
    if (currentStatus === 'connected' || currentStatus === 'disconnected') {
      resolve(currentStatus)
      return
    }
    if (currentStatus === 'error') {
      reject(new Error('Connection entered error state while waiting for terminal status'))
      return
    }

    let unsubscribe = () => {}
    const timeoutId = window.setTimeout(() => {
      unsubscribe()
      reject(new Error('Timeout waiting for terminal connection status'))
    }, timeoutMs)

    unsubscribe = useConnectionStore.subscribe((state) => {
      if (state.status === 'connected' || state.status === 'disconnected') {
        window.clearTimeout(timeoutId)
        unsubscribe()
        resolve(state.status)
      }
      else if (state.status === 'error') {
        window.clearTimeout(timeoutId)
        unsubscribe()
        reject(new Error('Connection entered error state while waiting for terminal status'))
      }
    })
  })
}

export function SettingsPage() {
  const [zapretDir, setZapretDir] = useState<string>('')
  const [resetDialogOpen, setResetDialogOpen] = useState(false)
  const [autostartEnabled, setAutostartEnabled] = useState(false)
  const [autostartLoading, setAutostartLoading] = useState(true)
  const [tcpDraft, setTcpDraft] = useState('')
  const [udpDraft, setUdpDraft] = useState('')
  const [tcpFocused, setTcpFocused] = useState(false)
  const [udpFocused, setUdpFocused] = useState(false)
  const isInitialLoadRef = useRef(true)

  const {
    config,
    loading,
    load,
    save,
    setGlobalPorts,
    setMinimizeToTray,
    setLaunchToTray,
    setConnectOnAutostart,
    reset,
  } = useConfigStore()
  const { isDownloading, progress, reset: resetDownload } = useDownloadStore()
  const { binariesOk, availableUpdates } = useAppStore()
  const { connect, disconnect } = useConnectionStore()

  useEffect(() => {
    let isMounted = true

    const init = async () => {
      try {
        await load()

        try {
          const dir = await tauri.getZapretDirectory()
          if (isMounted)
            setZapretDir(dir)
        }
        catch (e) {
          if (isMounted)
            toast.error(`Ошибка получения директории Zapret: ${e}`)
        }

        try {
          const autostart = await tauri.isAutostartEnabled()
          if (isMounted)
            setAutostartEnabled(autostart)
        }
        catch {
          if (isMounted)
            setAutostartEnabled(false)
        }
      }
      finally {
        if (isMounted) {
          isInitialLoadRef.current = false
          setAutostartLoading(false)
        }
      }
    }

    init().catch((e) => {
      if (isMounted)
        toast.error(`Ошибка инициализации настроек: ${e}`)
    })

    return () => {
      isMounted = false
    }
  }, [])

  useEffect(() => {
    if (config?.global_ports) {
      if (!tcpFocused) {
        setTcpDraft(config.global_ports.tcp)
      }
      if (!udpFocused) {
        setUdpDraft(config.global_ports.udp)
      }
    }
  }, [config?.global_ports, tcpFocused, udpFocused])

  useEffect(() => {
    let isMounted = true
    if (config && !isInitialLoadRef.current) {
      save().catch((e) => {
        if (isMounted)
          toast.error(`Ошибка сохранения настроек: ${e}`)
      })
    }
    return () => {
      isMounted = false
    }
  }, [config, save])

  const handleDownloadBinaries = async () => {
    const forceAll = binariesOk === true && availableUpdates.length === 0
    let shouldReconnect = false
    try {
      let stableStatus = useConnectionStore.getState().status
      if (stableStatus === 'connecting' || stableStatus === 'disconnecting') {
        stableStatus = await waitForTerminalConnectionStatus()
      }

      if (stableStatus === 'connected') {
        shouldReconnect = true
        await disconnect()
        await waitForConnectionStatus('disconnected')
      }

      await tauri.downloadBinaries(forceAll)
    }
    catch (e) {
      console.error(e)
      resetDownload()
      toast.error(`Ошибка загрузки файлов: ${e}`)
    }
    finally {
      if (shouldReconnect) {
        try {
          await connect()
          await waitForConnectionStatus('connected')
        }
        catch (e) {
          toast.error(`Ошибка восстановления подключения: ${e}`)
        }
      }
    }
  }

  const handleReset = async () => {
    try {
      await reset()
      await tauri.restoreDefaultFilters()
      setResetDialogOpen(false)
      toast.success('Настройки сброшены')
    }
    catch (e) {
      toast.error(`Ошибка сброса настроек: ${e}`)
    }
  }

  const handleAutostartChange = async (checked: boolean) => {
    setAutostartLoading(true)
    setAutostartEnabled(checked)
    try {
      await tauri.setAutostartEnabled(checked)
      toast.success(checked ? 'Автозапуск включен' : 'Автозапуск отключен')
    }
    catch (e) {
      setAutostartEnabled(!checked)
      toast.error(`Ошибка настройки автозапуска: ${e}`)
    }
    finally {
      setAutostartLoading(false)
    }
  }

  if (loading || !config) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="size-6 animate-spin" />
      </div>
    )
  }

  return (
    <ScrollArea className="h-full">
      <div className="space-y-6 p-6">
        <div>
          <h1 className="text-2xl font-medium">Настройки</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Глобальные параметры и управление файлами
          </p>
        </div>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Тема</CardTitle>
            <CardDescription>
              Внешний вид приложения
            </CardDescription>
          </CardHeader>
          <CardContent>
            <ThemeSwitcher />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Поведение</CardTitle>
            <CardDescription>
              Настройки запуска и закрытия приложения
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-3">
              <div className="flex items-center justify-between gap-4">
                <div className="space-y-0.5">
                  <Label htmlFor="autostart">Автозапуск с Windows</Label>
                  <p className="text-xs text-muted-foreground">
                    Приложение будет запускаться автоматически при входе в систему
                  </p>
                </div>
                <Switch
                  id="autostart"
                  checked={autostartEnabled}
                  disabled={autostartLoading}
                  onCheckedChange={handleAutostartChange}
                />
              </div>

              {autostartEnabled && (
                <div
                  className={cn(
                    'grid grid-rows-[1fr] opacity-100 transition-all duration-200 ease-out',
                  )}
                >
                  <div className="overflow-hidden">
                    <div className="flex items-center justify-between gap-4 border-l border-border/60 pl-4">
                      <div className="space-y-0.5">
                        <Label htmlFor="connect-on-autostart">Подключаться автоматически</Label>
                        <p className="text-xs text-muted-foreground">
                          При запуске из автозагрузки приложение будет сразу запускать подключение
                        </p>
                      </div>
                      <Switch
                        id="connect-on-autostart"
                        checked={config.connectOnAutostart ?? false}
                        disabled={autostartLoading}
                        onCheckedChange={setConnectOnAutostart}
                      />
                    </div>
                    <div className="mt-3 flex items-center justify-between gap-4 border-l border-border/60 pl-4">
                      <div className="space-y-0.5">
                        <Label htmlFor="launch-to-tray">Запускать свернутым в трей</Label>
                        <p className="text-xs text-muted-foreground">
                          При старте приложения основное окно будет скрыто, а доступ останется через иконку в трее
                        </p>
                      </div>
                      <Switch
                        id="launch-to-tray"
                        checked={config.launchToTray ?? false}
                        disabled={autostartLoading}
                        onCheckedChange={setLaunchToTray}
                      />
                    </div>
                  </div>
                </div>
              )}
            </div>

            <div className="flex items-center justify-between gap-4">
              <div className="space-y-0.5">
                <Label htmlFor="minimize-to-tray">Сворачивать в трей при закрытии</Label>
                <p className="text-xs text-muted-foreground">
                  При закрытии окно будет скрыто в системный трей вместо завершения работы
                </p>
              </div>
              <Switch
                id="minimize-to-tray"
                checked={config.minimizeToTray ?? true}
                onCheckedChange={setMinimizeToTray}
              />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Порты</CardTitle>
            <CardDescription>
              Глобальные порты для фильтрации трафика
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="tcpPortsInput">TCP порты</Label>
                <Input
                  id="tcpPortsInput"
                  value={tcpDraft}
                  onChange={e => setTcpDraft(e.target.value)}
                  onFocus={() => setTcpFocused(true)}
                  onBlur={async () => {
                    setTcpFocused(false)
                    if (isValidPortRange(tcpDraft)) {
                      const wasConnected = useConnectionStore.getState().status === 'connected'
                      setGlobalPorts({ ...config.global_ports, tcp: tcpDraft })
                      if (wasConnected) {
                        try {
                          await disconnect()
                          await waitForConnectionStatus('disconnected')
                          await connect()
                          await waitForConnectionStatus('connected')
                        }
                        catch (err) {
                          console.error('Failed to reconnect after port change:', err)
                          toast.error('Не удалось переподключиться с новыми портами')
                        }
                      }
                    }
                    else {
                      toast.error('Неверный формат портов. Пример: 80,443 или 1000-2000')
                    }
                  }}
                  placeholder="1-65535"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="udpPortsInput">UDP порты</Label>
                <Input
                  id="udpPortsInput"
                  value={udpDraft}
                  onChange={e => setUdpDraft(e.target.value)}
                  onFocus={() => setUdpFocused(true)}
                  onBlur={async () => {
                    setUdpFocused(false)
                    if (isValidPortRange(udpDraft)) {
                      const wasConnected = useConnectionStore.getState().status === 'connected'
                      setGlobalPorts({ ...config.global_ports, udp: udpDraft })
                      if (wasConnected) {
                        try {
                          await disconnect()
                          await waitForConnectionStatus('disconnected')
                          await connect()
                          await waitForConnectionStatus('connected')
                        }
                        catch (err) {
                          console.error('Failed to reconnect after port change:', err)
                          toast.error('Не удалось переподключиться с новыми портами')
                        }
                      }
                    }
                    else {
                      toast.error('Неверный формат портов. Пример: 80,443 или 1000-2000')
                    }
                  }}
                  placeholder="1-65535"
                />
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Бинарные файлы</CardTitle>
            <CardDescription>
              WinDivert, winws.exe, fake-файлы и списки
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center gap-2">
              <FolderOpen className="size-4 text-muted-foreground" />
              <span className="font-mono text-sm">{zapretDir}</span>
            </div>

            {binariesOk === false && (
              <Alert>
                <AlertTitle>Файлы не найдены</AlertTitle>
                <AlertDescription>
                  Необходимые файлы отсутствуют или повреждены
                </AlertDescription>
              </Alert>
            )}

            {binariesOk === true && availableUpdates.length > 0 && (
              <Alert className="border-yellow-600 bg-yellow-600/10">
                <AlertTitle>Доступно обновление файлов</AlertTitle>
                <AlertDescription>
                  {availableUpdates.length === 1
                    ? `Доступно обновление: ${availableUpdates[0]}`
                    : `Доступно обновление для ${availableUpdates.length} файлов`}
                </AlertDescription>
              </Alert>
            )}

            {binariesOk === true && availableUpdates.length === 0 && (
              <Alert className="border-green-600 bg-green-600/10">
                <AlertTitle>Файлы на месте</AlertTitle>
                <AlertDescription>
                  Все необходимые файлы найдены
                </AlertDescription>
              </Alert>
            )}

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
                  <Button
                    onClick={handleDownloadBinaries}
                    disabled={isDownloading}
                    variant={binariesOk === false ? 'default' : 'outline'}
                  >
                    <Download className="mr-2 size-4" />
                    {binariesOk === false
                      ? 'Загрузить'
                      : availableUpdates.length > 0
                        ? 'Обновить'
                        : 'Переустановить'}
                  </Button>
                )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Сброс</CardTitle>
            <CardDescription>
              Возврат к настройкам по умолчанию
            </CardDescription>
          </CardHeader>
          <CardContent>
            <AlertDialog open={resetDialogOpen} onOpenChange={setResetDialogOpen}>
              <AlertDialogTrigger asChild>
                <Button variant="destructive">
                  <RotateCcw className="mr-2 size-4" />
                  Сбросить настройки
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>Сбросить настройки?</AlertDialogTitle>
                  <AlertDialogDescription>
                    Все категории, стратегии, плейсхолдеры и фильтры будут удалены и заменены на значения по умолчанию. Это действие нельзя отменить.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>Отмена</AlertDialogCancel>
                  <AlertDialogAction onClick={handleReset} className="bg-destructive text-destructive-foreground hover:bg-destructive/90">
                    Сбросить
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </CardContent>
        </Card>
      </div>
    </ScrollArea>
  )
}
