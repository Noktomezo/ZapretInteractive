import { openUrl } from '@tauri-apps/plugin-opener'
import {
  Download,
  ExternalLink,
  Loader2,
  Package,
  RefreshCw,
  Shield,
  UserRound,
} from 'lucide-react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { useConfigStore } from '@/stores/config.store'
import { useUpdaterStore } from '@/stores/updater.store'

const APP_NAME = 'Zapret Interactive'
const APP_DEVELOPER = 'Noktomezo'
const APP_REPOSITORY_URL = 'https://github.com/Noktomezo/ZapretInteractive'
const APP_RELEASES_URL = 'https://github.com/Noktomezo/ZapretInteractive/releases'

const APP_LINKS = [
  { label: 'Исходники', value: 'Noktomezo/ZapretInteractive', href: APP_REPOSITORY_URL, icon: ExternalLink },
  { label: 'Релизы', value: 'Последние сборки и changelog', href: APP_RELEASES_URL, icon: Download },
  { label: 'Лицензия', value: 'MIT License', href: `${APP_REPOSITORY_URL}/blob/main/LICENSE`, icon: Shield },
]

const APP_FOUNDATIONS = [
  {
    label: 'zapret',
    value: 'Базовый DPI-bypass toolkit',
    href: 'https://github.com/bol-van/zapret',
  },
  {
    label: 'zapret-win-bundle',
    value: 'Windows bundle и служебные файлы',
    href: 'https://github.com/bol-van/zapret-win-bundle',
  },
  {
    label: 'Flexoki',
    value: 'Используемая тема и палитра интерфейса',
    href: 'https://github.com/kepano/flexoki',
  },
  {
    label: 'dnscrypt-proxy',
    value: 'Основа DNS-модуля и DoH-прокси',
    href: 'https://github.com/DNSCrypt/dnscrypt-proxy',
  },
  {
    label: 'tg-ws-proxy-rs',
    value: 'Основа Telegram WS Proxy модуля',
    href: 'https://github.com/valnesfjord/tg-ws-proxy-rs',
  },
]

function formatAboutTimestamp(value?: string) {
  if (!value) {
    return ''
  }

  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }

  return new Intl.DateTimeFormat('ru-RU', {
    day: '2-digit',
    month: '2-digit',
    year: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  }).format(date)
}

const PAGE_CARD_CLASS = 'gap-0! rounded-lg! border! border-border/60! bg-card! py-0! shadow-none! backdrop-blur-none!'

function AboutSectionHeader({
  icon: Icon,
  title,
  description,
  action,
  withDivider = true,
}: {
  icon: typeof Package
  title: React.ReactNode
  description: React.ReactNode
  action?: React.ReactNode
  withDivider?: boolean
}) {
  return (
    <CardHeader className={[
      'flex! flex-row! items-center! gap-3! p-4!',
      withDivider ? 'border-b border-border/60' : '',
    ].join(' ').trim()}
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

function MetaItem({
  icon: Icon,
  label,
  value,
}: {
  icon: typeof Package
  label: string
  value: string
}) {
  return (
    <div className="flex h-full flex-col rounded-xl border border-border/60 bg-muted/25 p-4">
      <div className="mb-2 flex items-center gap-2 text-muted-foreground">
        <Icon className="size-4" />
        <span className="text-xs uppercase tracking-[0.18em]">{label}</span>
      </div>
      <p className="text-sm font-medium break-all">{value}</p>
    </div>
  )
}

export function AboutPage() {
  const loading = useConfigStore(state => state.loading)
  const load = useConfigStore(state => state.load)
  const initUpdater = useUpdaterStore(state => state.init)
  const currentAppVersion = useUpdaterStore(state => state.currentVersion)
  const appUpdate = useUpdaterStore(state => state.availableUpdate)
  const appUpdateChecking = useUpdaterStore(state => state.checking)
  const appUpdateDownloading = useUpdaterStore(state => state.downloading)
  const appUpdateInstalling = useUpdaterStore(state => state.installing)
  const checkForAppUpdates = useUpdaterStore(state => state.checkForUpdates)
  const installAvailableAppUpdate = useUpdaterStore(state => state.installAvailableUpdate)

  useMountEffect(() => {
    let isMounted = true

    const init = async () => {
      try {
        await load()
        await initUpdater()
      }
      catch (e) {
        if (isMounted)
          toast.error(`Ошибка инициализации страницы: ${e instanceof Error ? e.message : String(e)}`)
      }
    }

    void init()

    return () => {
      isMounted = false
    }
  })

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

  const handleOpenExternal = async (url: string) => {
    try {
      await openUrl(url)
    }
    catch (e) {
      toast.error(`Не удалось открыть ссылку: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  if (loading) {
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
          <h1 className="text-2xl font-medium">О программе</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Информация о приложении, проверка обновлений и ссылки на проект
          </p>
        </div>

        <Card className={PAGE_CARD_CLASS}>
          <AboutSectionHeader
            icon={Package}
            title={APP_NAME}
            description="Desktop GUI для zapret-win-bundle с управлением стратегиями, фильтрами, плейсхолдерами и обновлениями"
          />
          <CardContent className="grid grid-cols-1 gap-3 p-4! sm:grid-cols-2">
            <div className="flex h-full flex-col rounded-xl border border-border/60 bg-muted/25 p-4">
              <div className="flex items-center justify-between gap-3">
                <div className="min-w-0">
                  <div className="mb-2 flex flex-wrap items-center gap-2 text-muted-foreground">
                    <Package className="size-4" />
                    <span className="text-xs uppercase tracking-[0.18em]">Версия</span>
                    <span
                      className={[
                        'rounded-[4px] border px-2 py-0.5 text-[10px] leading-none font-medium',
                        appUpdate && !appUpdateChecking && !appUpdateDownloading && !appUpdateInstalling
                          ? 'border-warning/30 bg-warning/12 text-warning'
                          : 'border-success/30 bg-success/10 text-success',
                      ].join(' ')}
                    >
                      {appUpdate && !appUpdateChecking && !appUpdateDownloading && !appUpdateInstalling ? 'Есть новее' : 'Последняя'}
                    </span>
                    <button
                      type="button"
                      className="inline-flex size-4 cursor-pointer items-center justify-center text-muted-foreground transition-colors hover:text-foreground disabled:pointer-events-none disabled:opacity-60"
                      aria-label="Проверить обновления"
                      onClick={() => { void handleManualAppUpdateCheck() }}
                      disabled={appUpdateChecking || appUpdateDownloading || appUpdateInstalling}
                    >
                      <RefreshCw className={appUpdateChecking ? 'size-3.5 animate-spin' : 'size-3.5'} />
                    </button>
                  </div>
                  <p className="font-mono text-sm font-medium break-all">{currentAppVersion ?? '...'}</p>
                </div>

                <div className="flex shrink-0 flex-wrap items-center gap-2">
                  {appUpdate && (
                    <Button
                      size="sm"
                      onClick={() => { void handleInstallAppUpdate() }}
                      disabled={appUpdateChecking || appUpdateDownloading || appUpdateInstalling}
                    >
                      <Download className="size-4" />
                      Обновить
                    </Button>
                  )}
                </div>
              </div>

              {appUpdate && (
                <div className="mt-3 rounded-lg border border-warning/30 bg-warning/12 p-3">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div>
                      <p className="text-sm font-medium">Доступна новая версия приложения</p>
                      <p className="mt-1 text-xs text-muted-foreground">
                        {appUpdate.version}
                        {appUpdate.date ? ` (${formatAboutTimestamp(appUpdate.date)})` : ''}
                      </p>
                    </div>
                  </div>
                </div>
              )}
            </div>

            <MetaItem icon={UserRound} label="Разработчик" value={APP_DEVELOPER} />
          </CardContent>
        </Card>

        <Card className={PAGE_CARD_CLASS}>
          <AboutSectionHeader
            icon={ExternalLink}
            title="Метаданные и ссылки"
            description="Базовая информация о проекте и полезные ссылки"
          />
          <CardContent className="space-y-4 p-4!">
            <div className="grid gap-3 sm:grid-cols-2">
              {APP_LINKS.map(({ label, value, href, icon: Icon }) => (
                <button
                  key={label}
                  type="button"
                  className="flex w-full cursor-pointer items-center justify-between rounded-xl border border-border/60 bg-muted/25 p-4 text-left transition-colors hover:bg-muted/45"
                  onClick={() => { void handleOpenExternal(href) }}
                >
                  <div>
                    <p className="text-sm font-medium">{label}</p>
                    <p className="mt-1 text-xs break-all text-muted-foreground">{value}</p>
                  </div>
                  <span className="ml-3 flex size-8 shrink-0 items-center justify-center rounded-lg border border-border/60 bg-background/70">
                    <Icon className="size-4" />
                  </span>
                </button>
              ))}
              {APP_FOUNDATIONS.map(item => (
                <button
                  key={item.label}
                  type="button"
                  className="flex w-full cursor-pointer items-center justify-between rounded-xl border border-border/60 bg-muted/25 p-4 text-left transition-colors hover:bg-muted/45"
                  onClick={() => { void handleOpenExternal(item.href) }}
                >
                  <div>
                    <p className="text-sm font-medium">{item.label}</p>
                    <p className="mt-1 text-xs break-all text-muted-foreground">{item.value}</p>
                  </div>
                  <span className="ml-3 flex size-8 shrink-0 items-center justify-center rounded-lg border border-border/60 bg-background/70">
                    <ExternalLink className="size-4" />
                  </span>
                </button>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    </LenisScrollArea>
  )
}
