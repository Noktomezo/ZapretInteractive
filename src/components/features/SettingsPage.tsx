import type { DiscordPresenceActivityType } from '@/lib/types'
import type { Theme } from '@/stores/theme.store'
import {
  AppWindow,
  CircleOff,
  Clapperboard,
  Download,
  Gamepad2,
  Headphones,
  Laptop,
  Loader2,
  MoonStar,
  Palette,
  RotateCcw,
  Router,
  SunMedium,
  Trophy,
} from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { MODULE_PAGE_CARD_CLASS, ModuleSectionHeader, ModuleSettingLabel } from '@/components/features/module-ui'
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
import { Card, CardContent } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
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

const THEME_OPTIONS_CONFIG: { value: Theme, labelKey: 'settings.theme.system' | 'settings.theme.light' | 'settings.theme.dark', icon: React.ComponentType<{ className?: string }> }[] = [
  { value: 'system', labelKey: 'settings.theme.system', icon: Laptop },
  { value: 'light', labelKey: 'settings.theme.light', icon: SunMedium },
  { value: 'dark', labelKey: 'settings.theme.dark', icon: MoonStar },
]

const DISCORD_PRESENCE_ACTIVITY_OPTIONS_CONFIG: { value: DiscordPresenceActivityType, labelKey: 'settings.behavior.discordPresence.playing' | 'settings.behavior.discordPresence.listening' | 'settings.behavior.discordPresence.watching' | 'settings.behavior.discordPresence.competing', icon: React.ComponentType<{ className?: string }> }[] = [
  { value: 'playing', labelKey: 'settings.behavior.discordPresence.playing', icon: Gamepad2 },
  { value: 'listening', labelKey: 'settings.behavior.discordPresence.listening', icon: Headphones },
  { value: 'watching', labelKey: 'settings.behavior.discordPresence.watching', icon: Clapperboard },
  { value: 'competing', labelKey: 'settings.behavior.discordPresence.competing', icon: Trophy },
]

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
  const { t } = useTranslation()
  const config = useConfigStore(state => state.config)
  const loading = useConfigStore(state => state.loading)
  const load = useConfigStore(state => state.load)
  const saveNow = useConfigStore(state => state.saveNow)
  const setGlobalPorts = useConfigStore(state => state.setGlobalPorts)
  const setCoreFileUpdatePromptsEnabled = useConfigStore(state => state.setCoreFileUpdatePromptsEnabled)
  const setAppAutoUpdatesEnabled = useConfigStore(state => state.setAppAutoUpdatesEnabled)
  const setDiscordPresenceEnabled = useConfigStore(state => state.setDiscordPresenceEnabled)
  const setDiscordPresenceActivityType = useConfigStore(state => state.setDiscordPresenceActivityType)
  const setMinimizeToTray = useConfigStore(state => state.setMinimizeToTray)
  const setLaunchToTray = useConfigStore(state => state.setLaunchToTray)
  const setConnectOnAutostart = useConfigStore(state => state.setConnectOnAutostart)
  const reset = useConfigStore(state => state.reset)
  const restartIfConnected = useConnectionStore(state => state.restartIfConnected)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)
  const theme = useThemeStore(state => state.theme)
  const setTheme = useThemeStore(state => state.setTheme)

  const themeOptions = THEME_OPTIONS_CONFIG.map(opt => ({
    value: opt.value,
    label: t(opt.labelKey),
    icon: opt.icon,
  }))
  const selectedThemeOption = themeOptions.find(option => option.value === theme) ?? themeOptions[0]

  const discordOptions: { value: 'none' | DiscordPresenceActivityType, label: string, icon: React.ComponentType<{ className?: string }> }[] = [
    { value: 'none', label: t('settings.behavior.discordPresence.off'), icon: CircleOff },
    ...DISCORD_PRESENCE_ACTIVITY_OPTIONS_CONFIG.map(opt => ({
      value: opt.value,
      label: t(opt.labelKey),
      icon: opt.icon,
    })),
  ]

  const selectedDiscordPresenceValue = (config?.discordPresenceEnabled ?? false)
    ? (config?.discordPresenceActivityType ?? 'playing')
    : 'none'
  const selectedDiscordPresenceOption = discordOptions.find(option => option.value === selectedDiscordPresenceValue) ?? discordOptions[0]

  const [autostartEnabled, setAutostartEnabled] = useState(false)
  const [autostartLoading, setAutostartLoading] = useState(false)
  const [autostartKnown, setAutostartKnown] = useState(true)
  const [resetDialogOpen, setResetDialogOpen] = useState(false)
  const [tcpDraft, setTcpDraft] = useState('')
  const [udpDraft, setUdpDraft] = useState('')
  const tcpFocusedRef = useRef(false)
  const udpFocusedRef = useRef(false)
  const saveTimeoutRef = useRef<number | null>(null)
  const pendingSectionRef = useRef<string | null>(null)

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
    void load()
    void refreshAutostartState(isMounted)
    return () => {
      isMounted = false
      if (saveTimeoutRef.current) {
        window.clearTimeout(saveTimeoutRef.current)
      }
    }
  })

  useEffect(() => {
    if (!tcpFocusedRef.current) {
      setTcpDraft(config?.global_ports.tcp ?? '')
    }
  }, [config?.global_ports.tcp])

  useEffect(() => {
    if (!udpFocusedRef.current) {
      setUdpDraft(config?.global_ports.udp ?? '')
    }
  }, [config?.global_ports.udp])

  const scheduleSave = (section: string) => {
    pendingSectionRef.current = section
    if (saveTimeoutRef.current) {
      window.clearTimeout(saveTimeoutRef.current)
    }
    saveTimeoutRef.current = window.setTimeout(async () => {
      try {
        await saveNow()
      }
      catch (e) {
        console.error('Failed to save settings:', e)
        toast.error(`Ошибка сохранения настроек: ${e instanceof Error ? e.message : String(e)}`)
      }
      finally {
        saveTimeoutRef.current = null
        pendingSectionRef.current = null
      }
    }, 400)
  }

  const handleReset = async () => {
    try {
      await reset()
      addConfigLog(t('settings.resetSection.successLog'))
      toast.success(t('settings.resetSection.successToast'))
      setResetDialogOpen(false)
      await restartIfConnected()
    }
    catch (e) {
      toast.error(t('settings.resetSection.errorToast', { error: e instanceof Error ? e.message : String(e) }))
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
        addConfigLog(t('settings.behavior.discordPresence.disabledLog'))
        toast.success(t('settings.behavior.discordPresence.disabledToast'))
      }
      else {
        addConfigLog(t('settings.behavior.discordPresence.changedLog', { value }))
        const label = discordOptions.find(option => option.value === value)?.label ?? value
        toast.success(t('settings.behavior.discordPresence.statusToast', { label }))
      }
    }
    catch (e) {
      setDiscordPresenceEnabled(previousEnabled)
      setDiscordPresenceActivityType(previous)
      toast.error(t('settings.behavior.discordPresence.errorToast', { error: e instanceof Error ? e.message : String(e) }))
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
          <h1 className="text-2xl font-medium">{t('settings.title')}</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {t('settings.subtitle')}
          </p>
        </div>

        <Card className={MODULE_PAGE_CARD_CLASS}>
          <ModuleSectionHeader
            icon={Palette}
            iconClassName="text-[#8B7EC8] dark:text-[#8B7EC8]"
            title={t('settings.theme.title')}
            description={t('settings.theme.description')}
            withDivider={false}
            action={(
              <div className="w-[10.5rem]">
                <Select
                  value={theme}
                  onValueChange={value => setTheme(value as Theme)}
                >
                  <SelectTrigger id="theme-select" className="w-full cursor-pointer">
                    <span className="flex items-center gap-2">
                      <selectedThemeOption.icon className="size-4 text-muted-foreground" />
                      <SelectValue placeholder={t('settings.theme.placeholder')}>
                        {selectedThemeOption.label}
                      </SelectValue>
                    </span>
                  </SelectTrigger>
                  <SelectContent>
                    {themeOptions.map(option => (
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
            )}
          />
        </Card>

        <Card className={MODULE_PAGE_CARD_CLASS}>
          <ModuleSectionHeader
            icon={Download}
            iconClassName="text-[#3AA99F] dark:text-[#3AA99F]"
            title={t('settings.updates.title')}
            description={t('settings.updates.description')}
          />
          <CardContent className="space-y-4 p-4!">
            <div className="flex items-center justify-between gap-4">
              <ModuleSettingLabel
                htmlFor="app-auto-updates"
                description={t('settings.updates.appAuto.description')}
              >
                {t('settings.updates.appAuto.title')}
              </ModuleSettingLabel>
              <Switch
                id="app-auto-updates"
                checked={config.appAutoUpdatesEnabled ?? true}
                onCheckedChange={handleAppAutoUpdatesChange}
              />
            </div>

            <div className="flex items-center justify-between gap-4">
              <ModuleSettingLabel
                htmlFor="core-file-update-prompts"
                description={t('settings.updates.corePrompts.description')}
              >
                {t('settings.updates.corePrompts.title')}
              </ModuleSettingLabel>
              <Switch
                id="core-file-update-prompts"
                checked={config.coreFileUpdatePromptsEnabled ?? true}
                onCheckedChange={handleCoreFileUpdatePromptsChange}
              />
            </div>
          </CardContent>
        </Card>

        <Card className={MODULE_PAGE_CARD_CLASS}>
          <ModuleSectionHeader
            icon={AppWindow}
            iconClassName="text-[#879A39] dark:text-[#879A39]"
            title={t('settings.behavior.title')}
            description={t('settings.behavior.description')}
          />
          <CardContent className="space-y-4 p-4!">
            <div className="space-y-3">
              <div className="flex items-center justify-between gap-4">
                <ModuleSettingLabel
                  htmlFor="autostart"
                  description={t('settings.behavior.autostart.description')}
                >
                  {t('settings.behavior.autostart.title')}
                </ModuleSettingLabel>
                <Switch
                  id="autostart"
                  checked={autostartEnabled}
                  disabled={autostartLoading || !autostartKnown}
                  onCheckedChange={handleAutostartChange}
                />
              </div>
              {!autostartKnown && (
                <p className="text-warning text-xs">
                  {t('settings.behavior.autostart.unknown')}
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
                    <ModuleSettingLabel
                      htmlFor="connect-on-autostart"
                      description={t('settings.behavior.connectOnAutostart.description')}
                    >
                      {t('settings.behavior.connectOnAutostart.title')}
                    </ModuleSettingLabel>
                    <Switch
                      id="connect-on-autostart"
                      checked={config.connectOnAutostart ?? false}
                      disabled={autostartLoading || !autostartEnabled}
                      onCheckedChange={handleConnectOnAutostartChange}
                    />
                  </div>
                  <div className="mt-3 flex items-center justify-between gap-4 border-l border-border/60 pl-4">
                    <ModuleSettingLabel
                      htmlFor="launch-to-tray"
                      description={t('settings.behavior.launchToTray.description')}
                    >
                      {t('settings.behavior.launchToTray.title')}
                    </ModuleSettingLabel>
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
              <ModuleSettingLabel
                htmlFor="minimize-to-tray"
                description={t('settings.behavior.minimizeToTray.description')}
              >
                {t('settings.behavior.minimizeToTray.title')}
              </ModuleSettingLabel>
              <Switch
                id="minimize-to-tray"
                checked={config.minimizeToTray ?? true}
                onCheckedChange={handleMinimizeToTrayChange}
              />
            </div>

            <div className="flex items-center justify-between gap-4">
              <ModuleSettingLabel
                htmlFor="discord-presence"
                description={t('settings.behavior.discordPresence.description')}
              >
                {t('settings.behavior.discordPresence.title')}
              </ModuleSettingLabel>
              <div className="w-[10.5rem]">
                <Select
                  value={selectedDiscordPresenceValue}
                  onValueChange={value => void handleDiscordPresenceChange(value as 'none' | DiscordPresenceActivityType)}
                >
                  <SelectTrigger id="discord-presence" className="w-full cursor-pointer">
                    <span className="flex items-center gap-2">
                      <selectedDiscordPresenceOption.icon className="size-4 text-muted-foreground" />
                      <SelectValue placeholder={t('settings.behavior.discordPresence.placeholder')}>
                        {selectedDiscordPresenceOption.label}
                      </SelectValue>
                    </span>
                  </SelectTrigger>
                  <SelectContent>
                    {discordOptions.map(option => (
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

        <Card className={MODULE_PAGE_CARD_CLASS}>
          <ModuleSectionHeader
            icon={Router}
            iconClassName="text-[#DA702C] dark:text-[#DA702C]"
            title={t('settings.ports.title')}
            description={t('settings.ports.description')}
          />
          <CardContent className="space-y-4 p-4!">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
              <ModuleSettingLabel
                htmlFor="tcpPortsInput"
                description={t('settings.ports.tcp.description')}
              >
                {t('settings.ports.tcp.title')}
              </ModuleSettingLabel>
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
                        addConfigLog(t('settings.ports.tcpChangedLog', { from: latestGlobalPorts.tcp, to: tcpDraft }))
                        await restartIfConnected()
                      }
                      catch (err) {
                        console.error('Failed to apply TCP port change:', err)
                        toast.error(t('settings.ports.tcpError'))
                      }
                    }
                    else {
                      toast.error(t('settings.ports.invalidFormat'))
                    }
                  }}
                  placeholder="1-65535"
                />
              </div>
            </div>
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
              <ModuleSettingLabel
                htmlFor="udpPortsInput"
                description={t('settings.ports.udp.description')}
              >
                {t('settings.ports.udp.title')}
              </ModuleSettingLabel>
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
                        addConfigLog(t('settings.ports.udpChangedLog', { from: latestGlobalPorts.udp, to: udpDraft }))
                        await restartIfConnected()
                      }
                      catch (err) {
                        console.error('Failed to apply UDP port change:', err)
                        toast.error(t('settings.ports.udpError'))
                      }
                    }
                    else {
                      toast.error(t('settings.ports.invalidFormat'))
                    }
                  }}
                  placeholder="1-65535"
                />
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className={MODULE_PAGE_CARD_CLASS}>
          <ModuleSectionHeader
            icon={RotateCcw}
            iconClassName="text-[#D14D41] dark:text-[#D14D41]"
            title={t('settings.resetSection.title')}
            description={t('settings.resetSection.description')}
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
                    {t('settings.resetSection.button')}
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>{t('settings.resetSection.dialogTitle')}</AlertDialogTitle>
                    <AlertDialogDescription>
                      {t('settings.resetSection.dialogDescription')}
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <AlertDialogFooter>
                    <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                    <AlertDialogAction variant="destructive" onClick={handleReset}>
                      {t('settings.resetSection.confirm')}
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
