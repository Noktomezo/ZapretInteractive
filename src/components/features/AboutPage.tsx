import { openUrl } from '@tauri-apps/plugin-opener'
import {
  Download,
  ExternalLink,
  FolderOpen,
  Github,
  Loader2,
  Package,
  RefreshCw,
  Shield,
  UserRound,
} from 'lucide-react'
import { useEffect } from 'react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { MarkdownContent } from '@/components/ui/markdown'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { runWithPausedConnection } from '@/lib/connection-flow'
import * as tauri from '@/lib/tauri'
import { useAppStore } from '@/stores/app.store'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'
import { useDownloadStore } from '@/stores/download.store'
import { useUpdaterStore } from '@/stores/updater.store'

const APP_NAME = 'Zapret Interactive'
const APP_DEVELOPER = 'Noktomezo'
const APP_LICENSE = 'MIT'
const APP_REPOSITORY_URL = 'https://github.com/Noktomezo/ZapretInteractive'
const APP_RELEASES_URL = 'https://github.com/Noktomezo/ZapretInteractive/releases'

const APP_LINKS = [
  { label: 'Исходники', value: 'Noktomezo/ZapretInteractive', href: APP_REPOSITORY_URL, icon: Github },
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
]

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
  const binariesOk = useAppStore(state => state.binariesOk)
  const availableUpdates = useAppStore(state => state.availableUpdates)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)
  const isDownloading = useDownloadStore(state => state.isDownloading)
  const progress = useDownloadStore(state => state.progress)
  const resetDownload = useDownloadStore(state => state.reset)
  const initUpdater = useUpdaterStore(state => state.init)
  const currentAppVersion = useUpdaterStore(state => state.currentVersion)
  const appUpdate = useUpdaterStore(state => state.availableUpdate)
  const appUpdateChecking = useUpdaterStore(state => state.checking)
  const appUpdateDownloading = useUpdaterStore(state => state.downloading)
  const appUpdateInstalling = useUpdaterStore(state => state.installing)
  const checkForAppUpdates = useUpdaterStore(state => state.checkForUpdates)
  const installAvailableAppUpdate = useUpdaterStore(state => state.installAvailableUpdate)
  const showBinaryStatusText = binariesOk === false || availableUpdates.length > 0
  const showBinaryDetails = showBinaryStatusText || Boolean(isDownloading && progress)

  useEffect(() => {
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
  }, [initUpdater, load])

  const handleManualAppUpdateCheck = async () => {
    try {
      await checkForAppUpdates({ manual: true, silent: false })
    }
    catch (e) {
      console.error('Failed to check app updates:', e)
    }
  }

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
      toast.error(`Ошибка загрузки файлов: ${e instanceof Error ? e.message : String(e)}`)
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

  const handleOpenAppDirectory = async () => {
    try {
      await tauri.openAppDirectory()
    }
    catch (e) {
      toast.error(`Ошибка открытия папки: ${e instanceof Error ? e.message : String(e)}`)
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
    <ScrollArea className="h-full min-h-0">
      <div className="space-y-6 p-6">
        <div>
          <h1 className="text-2xl font-medium">О программе</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Информация о приложении, проверка обновлений и ссылки на проект
          </p>
        </div>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">{APP_NAME}</CardTitle>
            <CardDescription>
              Desktop GUI для zapret-win-bundle с управлением стратегиями, фильтрами, плейсхолдерами и обновлениями
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex h-full flex-col rounded-xl border border-border/60 bg-muted/25 p-4">
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="mb-2 flex items-center gap-2 text-muted-foreground">
                    <Package className="size-4" />
                    <span className="text-xs uppercase tracking-[0.18em]">Версия</span>
                    <span
                      className={[
                        'rounded-[4px] border px-2 py-0.5 text-[10px] leading-none font-medium',
                        appUpdate && !appUpdateChecking && !appUpdateDownloading && !appUpdateInstalling
                          ? 'border-yellow-600/50 bg-yellow-600/10 text-amber-700 dark:text-amber-300'
                          : 'border-green-600/40 bg-green-600/10 text-green-700 dark:text-green-300',
                      ].join(' ')}
                    >
                      {appUpdate && !appUpdateChecking && !appUpdateDownloading && !appUpdateInstalling ? 'Есть новее' : 'Последняя'}
                    </span>
                  </div>
                  <p className="font-mono text-sm font-medium break-all">{currentAppVersion ?? '...'}</p>
                </div>

                <div className="flex shrink-0 flex-wrap items-center gap-2 self-start">
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

                  <div className="flex sm:justify-end">
                    <Button
                      variant="outline"
                      onClick={() => { void handleManualAppUpdateCheck() }}
                      disabled={appUpdateChecking || appUpdateDownloading || appUpdateInstalling}
                    >
                      <RefreshCw className={appUpdateChecking ? 'size-4 animate-spin' : 'size-4'} />
                      Проверить обновления
                    </Button>
                  </div>
                </div>
              </div>

              {appUpdate && (
                <div className="mt-3 rounded-lg border border-yellow-600/50 bg-yellow-600/10 p-3">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div>
                      <p className="text-sm font-medium">Доступна новая версия приложения</p>
                      <p className="mt-1 text-xs text-muted-foreground">
                        {appUpdate.version}
                        {appUpdate.date ? ` (${appUpdate.date})` : ''}
                      </p>
                    </div>
                  </div>

                  {appUpdate.notes && (
                    <div className="mt-3 rounded-md border border-yellow-600/25 bg-background/40">
                      <ScrollArea className="h-24">
                        <div className="p-3">
                          <MarkdownContent>
                            {appUpdate.notes}
                          </MarkdownContent>
                        </div>
                      </ScrollArea>
                    </div>
                  )}
                </div>
              )}
            </div>

            <div className="grid gap-3 sm:grid-cols-2">
              <MetaItem icon={UserRound} label="Разработчик" value={APP_DEVELOPER} />
              <MetaItem icon={Shield} label="Лицензия" value={APP_LICENSE} />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="space-y-4">
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <div className="mb-2 flex items-center gap-2">
                  <CardTitle className="text-lg">Файлы приложения</CardTitle>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <span
                        className={[
                          'rounded-[4px] border px-2 py-0.5 text-[10px] leading-none font-medium',
                          binariesOk === false
                            ? 'border-red-600/40 bg-red-600/10 text-red-700 dark:text-red-300'
                            : availableUpdates.length > 0
                              ? 'border-yellow-600/50 bg-yellow-600/10 text-amber-700 dark:text-amber-300'
                              : 'border-green-600/40 bg-green-600/10 text-green-700 dark:text-green-300',
                        ].join(' ')}
                      >
                        {binariesOk === false ? 'Не найдены' : availableUpdates.length > 0 ? 'Есть обновления' : 'Актуально'}
                      </span>
                    </TooltipTrigger>
                    <TooltipContent sideOffset={6}>
                      {binariesOk === false
                        ? 'Необходимые файлы отсутствуют или повреждены'
                        : availableUpdates.length === 1
                          ? `Доступно обновление: ${availableUpdates[0]}`
                          : availableUpdates.length > 1
                            ? `Доступно обновление для ${availableUpdates.length} файлов`
                            : 'Все необходимые файлы найдены'}
                    </TooltipContent>
                  </Tooltip>
                </div>
                <CardDescription>
                  WinDivert, winws.exe, fake-файлы и списки
                </CardDescription>
              </div>

              <div className="flex shrink-0 flex-wrap items-center gap-2 self-start">
                {!isDownloading && (
                  <Button
                    onClick={handleDownloadBinaries}
                    disabled={isDownloading}
                    variant={binariesOk === false ? 'default' : 'outline'}
                  >
                    <Download className="size-4" />
                    {binariesOk === false
                      ? 'Загрузить'
                      : availableUpdates.length > 0
                        ? 'Обновить'
                        : 'Переустановить'}
                  </Button>
                )}

                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="outline"
                      size="icon"
                      aria-label="Открыть папку приложения"
                      onClick={() => { void handleOpenAppDirectory() }}
                    >
                      <FolderOpen className="size-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent sideOffset={6}>Открыть папку приложения</TooltipContent>
                </Tooltip>
              </div>
            </div>
          </CardHeader>
          {showBinaryDetails && (
            <CardContent className="space-y-4 pt-0">
              {showBinaryStatusText && (
                <p className="text-sm text-muted-foreground">
                  {binariesOk === false
                    ? 'Необходимые файлы отсутствуют или повреждены'
                    : availableUpdates.length === 1
                      ? `Доступно обновление: ${availableUpdates[0]}`
                      : `Доступно обновление для ${availableUpdates.length} файлов`}
                </p>
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
                : null}
            </CardContent>
          )}
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Метаданные и ссылки</CardTitle>
            <CardDescription>
              Базовая информация о проекте и полезные ссылки
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-3 sm:grid-cols-2">
              {APP_LINKS.map(({ label, value, href, icon: Icon }) => (
                <button
                  key={label}
                  type="button"
                  className="flex w-full cursor-pointer items-start justify-between rounded-xl border border-border/60 bg-muted/25 p-4 text-left transition-colors hover:bg-muted/45"
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
                  className="flex w-full cursor-pointer items-start justify-between rounded-xl border border-border/60 bg-muted/25 p-4 text-left transition-colors hover:bg-muted/45"
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
    </ScrollArea>
  )
}
