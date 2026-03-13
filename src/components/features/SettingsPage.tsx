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
import { runWithPausedConnection } from '@/lib/connection-flow'
import * as tauri from '@/lib/tauri'
import { cn } from '@/lib/utils'
import { useAppStore } from '@/stores/app.store'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'
import { useDownloadStore } from '@/stores/download.store'
import { useUpdaterStore } from '@/stores/updater.store'

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

export function SettingsPage() {
  const [zapretDir, setZapretDir] = useState<string>('')
  const [resetDialogOpen, setResetDialogOpen] = useState(false)
  const [autostartEnabled, setAutostartEnabled] = useState(false)
  const [autostartKnown, setAutostartKnown] = useState(false)
  const [autostartLoading, setAutostartLoading] = useState(true)
  const [tcpDraft, setTcpDraft] = useState('')
  const [udpDraft, setUdpDraft] = useState('')
  const prevGlobalPortsRef = useRef<string | undefined>(undefined)
  const tcpFocusedRef = useRef(false)
  const udpFocusedRef = useRef(false)

  const config = useConfigStore(state => state.config)
  const loading = useConfigStore(state => state.loading)
  const load = useConfigStore(state => state.load)
  const saveNow = useConfigStore(state => state.saveNow)
  const scheduleSave = useConfigStore(state => state.scheduleSave)
  const setGlobalPorts = useConfigStore(state => state.setGlobalPorts)
  const setCoreFileUpdatePromptsEnabled = useConfigStore(state => state.setCoreFileUpdatePromptsEnabled)
  const setAppAutoUpdatesEnabled = useConfigStore(state => state.setAppAutoUpdatesEnabled)
  const setMinimizeToTray = useConfigStore(state => state.setMinimizeToTray)
  const setLaunchToTray = useConfigStore(state => state.setLaunchToTray)
  const setConnectOnAutostart = useConfigStore(state => state.setConnectOnAutostart)
  const reset = useConfigStore(state => state.reset)
  const isDownloading = useDownloadStore(state => state.isDownloading)
  const progress = useDownloadStore(state => state.progress)
  const resetDownload = useDownloadStore(state => state.reset)
  const binariesOk = useAppStore(state => state.binariesOk)
  const availableUpdates = useAppStore(state => state.availableUpdates)
  const restartIfConnected = useConnectionStore(state => state.restartIfConnected)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)
  const initUpdater = useUpdaterStore(state => state.init)
  const currentAppVersion = useUpdaterStore(state => state.currentVersion)
  const appUpdate = useUpdaterStore(state => state.availableUpdate)
  const appUpdateChecking = useUpdaterStore(state => state.checking)
  const appUpdateDownloading = useUpdaterStore(state => state.downloading)
  const appUpdateInstalling = useUpdaterStore(state => state.installing)
  const lastAppUpdateCheckError = useUpdaterStore(state => state.lastCheckError)
  const lastAppUpdateCheckedAt = useUpdaterStore(state => state.lastCheckedAt)
  const checkForAppUpdates = useUpdaterStore(state => state.checkForUpdates)
  const installAvailableAppUpdate = useUpdaterStore(state => state.installAvailableUpdate)

  const refreshAutostartState = async (isMounted = true) => {
    try {
      const autostart = await tauri.isAutostartEnabled()
      if (isMounted) {
        setAutostartEnabled(autostart)
        setAutostartKnown(true)
      }
    }
    catch (e) {
      if (isMounted) {
        setAutostartKnown(false)
        toast.error(`Не удалось определить статус автозапуска: ${e}`)
      }
    }
  }

  useEffect(() => {
    let isMounted = true

    const init = async () => {
      try {
        await load()
        await initUpdater()

        try {
          const dir = await tauri.getZapretDirectory()
          if (isMounted)
            setZapretDir(dir)
        }
        catch (e) {
          if (isMounted)
            toast.error(`Ошибка получения директории Zapret: ${e}`)
        }

        await refreshAutostartState(isMounted)
      }
      finally {
        if (isMounted)
          setAutostartLoading(false)
      }
    }

    init().catch((e) => {
      if (isMounted)
        toast.error(`Ошибка инициализации настроек: ${e}`)
    })

    return () => {
      isMounted = false
    }
  }, [initUpdater, load])

  useEffect(() => {
    if (config?.global_ports) {
      const currentPortsJson = JSON.stringify(config.global_ports)
      if (prevGlobalPortsRef.current === currentPortsJson) {
        return
      }
      prevGlobalPortsRef.current = currentPortsJson
      if (!tcpFocusedRef.current) {
        setTcpDraft(config.global_ports.tcp)
      }
      if (!udpFocusedRef.current) {
        setUdpDraft(config.global_ports.udp)
      }
    }
  }, [config?.global_ports])

  const handleDownloadBinaries = async () => {
    const forceAll = binariesOk === true && availableUpdates.length === 0
    try {
      addConfigLog(forceAll
        ? 'запущена переустановка файлов приложения'
        : 'запущено обновление файлов приложения')
      await runWithPausedConnection(async () => {
        await tauri.downloadBinaries(forceAll)
      })
    }
    catch (e) {
      console.error(e)
      resetDownload()
      toast.error(`Ошибка загрузки файлов: ${e}`)
    }
  }

  const handleReset = async () => {
    try {
      await reset()
      await tauri.ensureManagedFiles()
      setResetDialogOpen(false)
      addConfigLog('конфигурация сброшена к значениям по умолчанию')
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
      addConfigLog(checked ? 'автозапуск Windows включён' : 'автозапуск Windows отключён')
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

  const handleConnectOnAutostartChange = (checked: boolean) => {
    setConnectOnAutostart(checked)
    scheduleSave('connect-on-autostart')
    addConfigLog(checked
      ? 'автоподключение из автозагрузки включено'
      : 'автоподключение из автозагрузки отключено')
  }

  const handleLaunchToTrayChange = (checked: boolean) => {
    setLaunchToTray(checked)
    scheduleSave('launch-to-tray')
    addConfigLog(checked ? 'запуск в трей включён' : 'запуск в трей отключён')
  }

  const handleMinimizeToTrayChange = (checked: boolean) => {
    setMinimizeToTray(checked)
    scheduleSave('minimize-to-tray')
    addConfigLog(checked
      ? 'сворачивание в трей при закрытии включено'
      : 'сворачивание в трей при закрытии отключено')
  }

  const handleCoreFileUpdatePromptsChange = (checked: boolean) => {
    setCoreFileUpdatePromptsEnabled(checked)
    scheduleSave('core-file-update-prompts')
    addConfigLog(checked
      ? 'автопредложения обновления winws/fake файлов включены'
      : 'автопредложения обновления winws/fake файлов отключены')
  }

  const handleAppAutoUpdatesChange = async (checked: boolean) => {
    if (!config) {
      return
    }

    const previous = config.appAutoUpdatesEnabled ?? true
    setAppAutoUpdatesEnabled(checked)
    try {
      await saveNow()
      addConfigLog(checked
        ? 'автоматическая проверка обновлений приложения включена'
        : 'автоматическая проверка обновлений приложения отключена')
      toast.success(checked ? 'Автообновления приложения включены' : 'Автообновления приложения отключены')
    }
    catch (e) {
      setAppAutoUpdatesEnabled(previous)
      toast.error(`Ошибка настройки автообновлений приложения: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  const handleManualAppUpdateCheck = async () => {
    try {
      await checkForAppUpdates({ manual: true, silent: false })
    }
    catch (e) {
      console.error('Failed to check app updates:', e)
    }
  }

  const handleInstallAppUpdate = async () => {
    try {
      await installAvailableAppUpdate()
    }
    catch (e) {
      console.error('Failed to install app update:', e)
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
    <ScrollArea className="h-full min-h-0">
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
            <CardTitle className="text-lg">Обновление приложения</CardTitle>
            <CardDescription>
              Автоматическая и ручная проверка новых версий Zapret Interactive
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between gap-4">
              <div className="space-y-0.5">
                <Label htmlFor="app-auto-updates">Автоматически проверять обновления приложения</Label>
                <p className="text-xs text-muted-foreground">
                  При запуске и каждые 5 минут приложение будет проверять наличие новой версии
                </p>
              </div>
              <Switch
                id="app-auto-updates"
                checked={config.appAutoUpdatesEnabled ?? true}
                disabled={appUpdateChecking || appUpdateDownloading || appUpdateInstalling}
                onCheckedChange={handleAppAutoUpdatesChange}
              />
            </div>

            <div className="space-y-1">
              <p className="text-sm font-medium">
                Текущая версия:
                {' '}
                <span className="font-mono">{currentAppVersion ?? '...'}</span>
              </p>
              {appUpdateChecking && (
                <p className="text-xs text-muted-foreground">Проверяю наличие новой версии...</p>
              )}
              {!appUpdateChecking && appUpdateInstalling && (
                <p className="text-xs text-muted-foreground">Устанавливаю обновление приложения...</p>
              )}
              {!appUpdateChecking && !appUpdateInstalling && appUpdateDownloading && (
                <p className="text-xs text-muted-foreground">Загружаю обновление приложения...</p>
              )}
              {!appUpdateChecking && !appUpdateDownloading && !appUpdateInstalling && appUpdate && (
                <p className="text-xs text-amber-600 dark:text-amber-400">
                  Доступна версия
                  {' '}
                  {appUpdate.version}
                </p>
              )}
              {!appUpdateChecking && !appUpdateDownloading && !appUpdateInstalling && !appUpdate && lastAppUpdateCheckError && (
                <p className="text-xs text-red-600 dark:text-red-400">
                  Ошибка проверки:
                  {' '}
                  {lastAppUpdateCheckError}
                </p>
              )}
              {!appUpdateChecking && !appUpdateDownloading && !appUpdateInstalling && !appUpdate && !lastAppUpdateCheckError && lastAppUpdateCheckedAt && (
                <p className="text-xs text-muted-foreground">Новых версий не найдено</p>
              )}
            </div>

            {appUpdate && (
              <Alert className="border-yellow-600 bg-yellow-600/10">
                <AlertTitle>Доступна новая версия приложения</AlertTitle>
                <AlertDescription className="space-y-2">
                  <p>
                    Доступна версия
                    {' '}
                    {appUpdate.version}
                    {appUpdate.date ? ` (${appUpdate.date})` : ''}
                  </p>
                  {appUpdate.notes && (
                    <p className="line-clamp-4 whitespace-pre-wrap text-xs text-muted-foreground">
                      {appUpdate.notes}
                    </p>
                  )}
                </AlertDescription>
              </Alert>
            )}

            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                onClick={() => { void handleManualAppUpdateCheck() }}
                disabled={appUpdateChecking || appUpdateDownloading || appUpdateInstalling}
              >
                Проверить обновления
              </Button>
              {appUpdate && (
                <Button
                  onClick={() => { void handleInstallAppUpdate() }}
                  disabled={appUpdateChecking || appUpdateDownloading || appUpdateInstalling}
                >
                  Установить обновление
                </Button>
              )}
            </div>
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
                  disabled={autostartLoading || !autostartKnown}
                  onCheckedChange={handleAutostartChange}
                />
              </div>
              {!autostartKnown && (
                <p className="text-xs text-amber-600 dark:text-amber-400">
                  Не удалось определить текущий статус автозапуска. Перезайдите на страницу позже.
                </p>
              )}

              <div
                className={cn(
                  autostartEnabled ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0',
                  'grid transition-all duration-200 ease-out',
                )}
                aria-hidden={!autostartEnabled}
                hidden={!autostartEnabled}
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
                      disabled={autostartLoading || !autostartEnabled}
                      onCheckedChange={handleConnectOnAutostartChange}
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
                      disabled={autostartLoading || !autostartEnabled}
                      onCheckedChange={handleLaunchToTrayChange}
                    />
                  </div>
                </div>
              </div>
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
                onCheckedChange={handleMinimizeToTrayChange}
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
                  onFocus={() => { tcpFocusedRef.current = true }}
                  onBlur={async () => {
                    tcpFocusedRef.current = false
                    const latestGlobalPorts = useConfigStore.getState().config?.global_ports ?? config.global_ports
                    if (latestGlobalPorts.tcp === tcpDraft) {
                      return
                    }
                    if (isValidPortRange(tcpDraft)) {
                      setGlobalPorts({ ...latestGlobalPorts, tcp: tcpDraft })
                      try {
                        await saveNow()
                        addConfigLog(`TCP порты изменены с ${latestGlobalPorts.tcp} на ${tcpDraft}`)
                        await restartIfConnected()
                      }
                      catch (err) {
                        console.error('Failed to apply TCP port change:', err)
                        toast.error('Не удалось применить новые TCP порты')
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
                  onFocus={() => { udpFocusedRef.current = true }}
                  onBlur={async () => {
                    udpFocusedRef.current = false
                    const latestGlobalPorts = useConfigStore.getState().config?.global_ports ?? config.global_ports
                    if (latestGlobalPorts.udp === udpDraft) {
                      return
                    }
                    if (isValidPortRange(udpDraft)) {
                      setGlobalPorts({ ...latestGlobalPorts, udp: udpDraft })
                      try {
                        await saveNow()
                        addConfigLog(`UDP порты изменены с ${latestGlobalPorts.udp} на ${udpDraft}`)
                        await restartIfConnected()
                      }
                      catch (err) {
                        console.error('Failed to apply UDP port change:', err)
                        toast.error('Не удалось применить новые UDP порты')
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

            <div className="flex items-center justify-between gap-4">
              <div className="space-y-0.5">
                <Label htmlFor="core-file-update-prompts">Предлагать обновления winws/fake файлов</Label>
                <p className="text-xs text-muted-foreground">
                  Фоновые проверки обновлений останутся, но можно скрыть автоматические предложения обновить файлы
                </p>
              </div>
              <Switch
                id="core-file-update-prompts"
                checked={config.coreFileUpdatePromptsEnabled ?? true}
                onCheckedChange={handleCoreFileUpdatePromptsChange}
              />
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
