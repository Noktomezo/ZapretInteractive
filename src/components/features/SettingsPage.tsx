import type { DiscordPresenceActivityType, WindowMaterial, WindowMaterialCapabilities } from '@/lib/types'
import type { Theme } from '@/stores/theme.store'
import {
  AppWindow,
  ArrowLeftRight,
  BellRing,
  CircleOff,
  Clapperboard,
  Download,
  Gamepad2,
  Headphones,
  Laptop,
  Layers3,
  Loader2,
  Minimize2,
  MoonStar,
  Palette,
  PanelTop,
  PlugZap,
  Power,
  Radar,
  RefreshCw,
  RotateCcw,
  Router,
  Sparkles,
  SunMedium,
  Trophy,
} from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
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
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { useMountEffect } from '@/hooks/use-mount-effect'
import * as tauri from '@/lib/tauri'
import { cn } from '@/lib/utils'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'
import { useThemeStore } from '@/stores/theme.store'

const RANGE_RE = /^\d+-\d+$/
const PORT_RE = /^\d+$/
const THEME_OPTIONS: { value: Theme, label: string, icon: React.ComponentType<{ className?: string }> }[] = [
  { value: 'light', label: 'Светлая', icon: SunMedium },
  { value: 'dark', label: 'Тёмная', icon: MoonStar },
  { value: 'system', label: 'Системная', icon: Laptop },
]
const WINDOW_MATERIAL_OPTIONS: { value: WindowMaterial, label: string, icon: React.ComponentType<{ className?: string }> }[] = [
  { value: 'none', label: 'Нет', icon: CircleOff },
  { value: 'acrylic', label: 'Акрил', icon: Layers3 },
  { value: 'mica', label: 'Mica', icon: Sparkles },
  { value: 'tabbed', label: 'Tabbed', icon: PanelTop },
]
const DISCORD_PRESENCE_ACTIVITY_OPTIONS: { value: DiscordPresenceActivityType, label: string, icon: React.ComponentType<{ className?: string }> }[] = [
  { value: 'playing', label: 'Играет', icon: Gamepad2 },
  { value: 'listening', label: 'Слушает', icon: Headphones },
  { value: 'watching', label: 'Смотрит', icon: Clapperboard },
  { value: 'competing', label: 'Соревнуется', icon: Trophy },
]
const DISCORD_PRESENCE_SELECT_OPTIONS: { value: 'none' | DiscordPresenceActivityType, label: string, icon: React.ComponentType<{ className?: string }> }[] = [
  { value: 'none', label: 'Нет', icon: CircleOff },
  ...DISCORD_PRESENCE_ACTIVITY_OPTIONS,
]

const PAGE_CARD_CLASS = 'gap-0! rounded-lg! border! border-border/60! bg-card! py-0! shadow-none! backdrop-blur-none!'

function SettingsSectionHeader({
  icon: Icon,
  title,
  description,
  action,
  withDivider = true,
}: {
  icon: React.ComponentType<{ className?: string }>
  title: React.ReactNode
  description: React.ReactNode
  action?: React.ReactNode
  withDivider?: boolean
}) {
  return (
    <CardHeader className={cn(
      'flex! flex-row! items-center! gap-3! p-4!',
      withDivider && 'border-b border-border/60',
    )}
    >
      <div className="text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-muted/25">
        <Icon className="size-4" />
      </div>
      <div className="min-w-0 flex-1">
        <CardTitle className="font-sans text-sm leading-5 font-normal tracking-normal">{title}</CardTitle>
        <CardDescription className="mt-1 text-xs leading-4">{description}</CardDescription>
      </div>
      {action ? <CardAction className="self-center">{action}</CardAction> : null}
    </CardHeader>
  )
}

function SettingLabel({
  htmlFor,
  icon: Icon,
  description,
  children,
}: {
  htmlFor: string
  icon: React.ComponentType<{ className?: string }>
  description?: React.ReactNode
  children: React.ReactNode
}) {
  return (
    <div className="flex items-center gap-3">
      <div className="text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-muted/25">
        <Icon className="size-4" />
      </div>
      <div className="min-w-0 flex-1">
        <Label htmlFor={htmlFor} className="text-sm leading-5 font-normal">
          {children}
        </Label>
        {description
          ? <p className="mt-1 text-xs leading-4 text-muted-foreground">{description}</p>
          : null}
      </div>
    </div>
  )
}

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
  const [resetDialogOpen, setResetDialogOpen] = useState(false)
  const [autostartEnabled, setAutostartEnabled] = useState(false)
  const [autostartKnown, setAutostartKnown] = useState(false)
  const [autostartLoading, setAutostartLoading] = useState(true)
  const [tcpDraft, setTcpDraft] = useState('')
  const [udpDraft, setUdpDraft] = useState('')
  const [windowMaterialCapabilities, setWindowMaterialCapabilities] = useState<WindowMaterialCapabilities>({
    acrylic: true,
    mica: false,
    tabbed: false,
  })
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
  const setDiscordPresenceEnabled = useConfigStore(state => state.setDiscordPresenceEnabled)
  const setDiscordPresenceActivityType = useConfigStore(state => state.setDiscordPresenceActivityType)
  const setWindowMaterial = useConfigStore(state => state.setWindowMaterial)
  const setMinimizeToTray = useConfigStore(state => state.setMinimizeToTray)
  const setLaunchToTray = useConfigStore(state => state.setLaunchToTray)
  const setConnectOnAutostart = useConfigStore(state => state.setConnectOnAutostart)
  const reset = useConfigStore(state => state.reset)
  const restartIfConnected = useConnectionStore(state => state.restartIfConnected)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)
  const theme = useThemeStore(state => state.theme)
  const setTheme = useThemeStore(state => state.setTheme)
  const selectedTheme = THEME_OPTIONS.find(option => option.value === theme) ?? THEME_OPTIONS[2]
  const selectedWindowMaterial = WINDOW_MATERIAL_OPTIONS.find(option => option.value === (config?.windowMaterial ?? 'none')) ?? WINDOW_MATERIAL_OPTIONS[0]
  const selectedDiscordPresenceValue = (config?.discordPresenceEnabled ?? false)
    ? (config?.discordPresenceActivityType ?? 'playing')
    : 'none'
  const selectedDiscordPresenceOption = DISCORD_PRESENCE_SELECT_OPTIONS.find(option => option.value === selectedDiscordPresenceValue) ?? DISCORD_PRESENCE_SELECT_OPTIONS[0]

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

  useMountEffect(() => {
    let isMounted = true

    const init = async () => {
      try {
        await load()
        const materialCapabilities = await tauri.getWindowMaterialCapabilities()
        if (isMounted) {
          setWindowMaterialCapabilities(materialCapabilities)
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
  })

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
    const previous = config?.appAutoUpdatesEnabled ?? true
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

  const handleDiscordPresenceChange = async (value: 'none' | DiscordPresenceActivityType) => {
    const previousEnabled = config?.discordPresenceEnabled ?? false
    const previous = config?.discordPresenceActivityType ?? 'playing'
    const nextEnabled = value !== 'none'
    setDiscordPresenceEnabled(nextEnabled)
    if (value !== 'none') {
      setDiscordPresenceActivityType(value)
    }
    try {
      await saveNow()
      if (!nextEnabled) {
        addConfigLog('Discord Rich Presence отключён')
        toast.success('Discord Rich Presence отключён')
      }
      else {
        addConfigLog(`тип активности Discord Rich Presence изменён: ${value}`)
        toast.success(`Discord статус: ${DISCORD_PRESENCE_ACTIVITY_OPTIONS.find(option => option.value === value)?.label ?? value}`)
      }
    }
    catch (e) {
      setDiscordPresenceEnabled(previousEnabled)
      setDiscordPresenceActivityType(previous)
      toast.error(`Ошибка настройки Discord Rich Presence: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  const handleWindowMaterialChange = async (value: WindowMaterial) => {
    const previous = config?.windowMaterial ?? 'none'
    setWindowMaterial(value)

    try {
      await tauri.setWindowMaterial(value)
      addConfigLog(`материал изменён: ${WINDOW_MATERIAL_OPTIONS.find(option => option.value === value)?.label ?? value}`)
      toast.success(`Материал: ${WINDOW_MATERIAL_OPTIONS.find(option => option.value === value)?.label ?? value}`)
    }
    catch (e) {
      setWindowMaterial(previous)
      try {
        await tauri.setWindowMaterial(previous)
      }
      catch (restoreError) {
        console.error('Failed to restore window material state:', restoreError)
      }
      toast.error(`Ошибка настройки материала: ${e instanceof Error ? e.message : String(e)}`)
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
    <LenisScrollArea className="h-full min-h-0">
      <div className="space-y-6 p-6">
        <div>
          <h1 className="text-2xl font-medium">Настройки</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Глобальные параметры приложения
          </p>
        </div>

        <Card className={PAGE_CARD_CLASS}>
          <SettingsSectionHeader
            icon={Palette}
            title="Тема"
            description="Внешний вид приложения"
          />
          <CardContent className="space-y-4 p-4!">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
              <SettingLabel
                htmlFor="theme-select"
                icon={SunMedium}
                description="Режим отображения интерфейса приложения"
              >
                Режим
              </SettingLabel>
              <div className="w-full sm:w-[11rem]">
                <Select value={theme} onValueChange={value => setTheme(value as Theme)}>
                  <SelectTrigger id="theme-select" className="w-full cursor-pointer">
                    <span className="flex items-center gap-2">
                      <selectedTheme.icon className="size-4 text-muted-foreground" />
                      <SelectValue placeholder="Выберите режим">{selectedTheme.label}</SelectValue>
                    </span>
                  </SelectTrigger>
                  <SelectContent>
                    {THEME_OPTIONS.map(option => (
                      <SelectItem key={option.value} value={option.value}>
                        <span className="flex items-center gap-2">
                          <option.icon className="size-4 text-muted-foreground" />
                          <span>{option.label}</span>
                        </span>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>

            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
              <SettingLabel
                htmlFor="window-material"
                icon={Layers3}
                description="Материал сайдбара и тайтлбара"
              >
                Материал
              </SettingLabel>
              <div className="w-full sm:w-[11rem]">
                <Select value={config.windowMaterial ?? 'none'} onValueChange={value => void handleWindowMaterialChange(value as WindowMaterial)}>
                  <SelectTrigger id="window-material" className="w-full cursor-pointer">
                    <span className="flex items-center gap-2">
                      <selectedWindowMaterial.icon className="size-4 text-muted-foreground" />
                      <SelectValue placeholder="Выберите материал">{selectedWindowMaterial.label}</SelectValue>
                    </span>
                  </SelectTrigger>
                  <SelectContent>
                    {WINDOW_MATERIAL_OPTIONS.map(option => (
                      <SelectItem
                        key={option.value}
                        value={option.value}
                        disabled={option.value === 'mica'
                          ? !windowMaterialCapabilities.mica
                          : option.value === 'tabbed'
                            ? !windowMaterialCapabilities.tabbed
                            : false}
                      >
                        <span className="flex items-center gap-2">
                          <option.icon className="size-4 text-muted-foreground" />
                          <span>{option.label}</span>
                        </span>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className={PAGE_CARD_CLASS}>
          <SettingsSectionHeader
            icon={Download}
            title="Обновления"
            description="Настройки фоновых проверок и автоматических предложений"
          />
          <CardContent className="space-y-4 p-4!">
            <div className="flex items-center justify-between gap-4">
              <SettingLabel
                htmlFor="app-auto-updates"
                icon={RefreshCw}
                description="При запуске и каждые 30 секунд приложение будет проверять наличие новой версии"
              >
                Автоматически проверять обновления приложения
              </SettingLabel>
              <Switch
                id="app-auto-updates"
                checked={config.appAutoUpdatesEnabled ?? true}
                onCheckedChange={handleAppAutoUpdatesChange}
              />
            </div>

            <div className="flex items-center justify-between gap-4">
              <SettingLabel
                htmlFor="core-file-update-prompts"
                icon={BellRing}
                description="Отключает только предложения обновить файлы, не саму проверку"
              >
                Уведомлять об обновлениях критических файлов
              </SettingLabel>
              <Switch
                id="core-file-update-prompts"
                checked={config.coreFileUpdatePromptsEnabled ?? true}
                onCheckedChange={handleCoreFileUpdatePromptsChange}
              />
            </div>
          </CardContent>
        </Card>

        <Card className={PAGE_CARD_CLASS}>
          <SettingsSectionHeader
            icon={AppWindow}
            title="Поведение"
            description="Настройки запуска и закрытия приложения"
          />
          <CardContent className="space-y-4 p-4!">
            <div className="space-y-3">
              <div className="flex items-center justify-between gap-4">
                <SettingLabel
                  htmlFor="autostart"
                  icon={Power}
                  description="Приложение будет запускаться автоматически при входе в систему"
                >
                  Автозапуск с Windows
                </SettingLabel>
                <Switch
                  id="autostart"
                  checked={autostartEnabled}
                  disabled={autostartLoading || !autostartKnown}
                  onCheckedChange={handleAutostartChange}
                />
              </div>
              {!autostartKnown && (
                <p className="text-warning text-xs">
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
                    <SettingLabel
                      htmlFor="connect-on-autostart"
                      icon={PlugZap}
                      description="При запуске из автозагрузки приложение будет сразу запускать подключение"
                    >
                      Подключаться автоматически
                    </SettingLabel>
                    <Switch
                      id="connect-on-autostart"
                      checked={config.connectOnAutostart ?? false}
                      disabled={autostartLoading || !autostartEnabled}
                      onCheckedChange={handleConnectOnAutostartChange}
                    />
                  </div>
                  <div className="mt-3 flex items-center justify-between gap-4 border-l border-border/60 pl-4">
                    <SettingLabel
                      htmlFor="launch-to-tray"
                      icon={AppWindow}
                      description="При старте приложения основное окно будет скрыто, а доступ останется через иконку в трее"
                    >
                      Запускать свернутым в трей
                    </SettingLabel>
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
              <SettingLabel
                htmlFor="minimize-to-tray"
                icon={Minimize2}
                description="При закрытии окно будет скрыто в системный трей вместо завершения работы"
              >
                Сворачивать в трей при закрытии
              </SettingLabel>
              <Switch
                id="minimize-to-tray"
                checked={config.minimizeToTray ?? true}
                onCheckedChange={handleMinimizeToTrayChange}
              />
            </div>

            <div className="flex items-center justify-between gap-4">
              <SettingLabel
                htmlFor="discord-presence"
                icon={Gamepad2}
                description="Показывает текущую страницу и статус подключения в Discord"
              >
                Discord Rich Presence
              </SettingLabel>
              <div className="w-[10.5rem]">
                <Select
                  value={selectedDiscordPresenceValue}
                  onValueChange={value => void handleDiscordPresenceChange(value as 'none' | DiscordPresenceActivityType)}
                >
                  <SelectTrigger id="discord-presence" className="w-full cursor-pointer">
                    <span className="flex items-center gap-2">
                      <selectedDiscordPresenceOption.icon className="size-4 text-muted-foreground" />
                      <SelectValue placeholder="Выберите статус">
                        {selectedDiscordPresenceOption.label}
                      </SelectValue>
                    </span>
                  </SelectTrigger>
                  <SelectContent>
                    {DISCORD_PRESENCE_SELECT_OPTIONS.map(option => (
                      <SelectItem key={option.value} value={option.value}>
                        <span className="flex items-center gap-2">
                          <option.icon className="size-4 text-muted-foreground" />
                          <span>{option.label}</span>
                        </span>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className={PAGE_CARD_CLASS}>
          <SettingsSectionHeader
            icon={Router}
            title="Порты"
            description="Глобальные порты для фильтрации трафика"
          />
          <CardContent className="space-y-4 p-4!">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
              <SettingLabel
                htmlFor="tcpPortsInput"
                icon={ArrowLeftRight}
                description="Порты TCP-трафика, на которые применяются стратегии фильтрации."
              >
                TCP порты
              </SettingLabel>
              <div className="w-full sm:w-[11rem]">
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
            </div>
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
              <SettingLabel
                htmlFor="udpPortsInput"
                icon={Radar}
                description="Порты UDP-трафика, на которые применяются стратегии фильтрации."
              >
                UDP порты
              </SettingLabel>
              <div className="w-full sm:w-[11rem]">
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

        <Card className={PAGE_CARD_CLASS}>
          <SettingsSectionHeader
            icon={RotateCcw}
            title="Сброс"
            description="Возврат к настройкам по умолчанию"
            withDivider={false}
            action={(
              <AlertDialog open={resetDialogOpen} onOpenChange={setResetDialogOpen}>
                <AlertDialogTrigger asChild>
                  <Button
                    variant="destructive"
                    size="sm"
                    className="border border-destructive/35 bg-destructive/72 shadow-none hover:bg-destructive/82 hover:shadow-none dark:border-destructive/30 dark:bg-destructive/58 dark:hover:bg-destructive/68"
                  >
                    <RotateCcw className="size-4" />
                    Сбросить
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
                    <AlertDialogAction variant="destructive" onClick={handleReset}>
                      Сбросить
                    </AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            )}
          />
        </Card>
      </div>
    </LenisScrollArea>
  )
}
