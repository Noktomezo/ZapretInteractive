import { Link } from '@tanstack/react-router'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ArrowLeft, Copy, KeyRound, Link2, Loader2, Power, RefreshCw, Send, ShieldCheck } from 'lucide-react'
import { useRef, useState } from 'react'
import { toast } from 'sonner'
import { MODULE_PAGE_CARD_CLASS, ModuleSectionHeader, ModuleSettingLabel } from '@/components/features/module-ui'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import { useTgWsProxyModule } from '@/hooks/use-tg-ws-proxy-module'
import { buildTgWsProxyHttpLink, generateTgWsProxySecret, normalizeTgWsProxySecret } from '@/lib/tg-ws-proxy'
import { cn } from '@/lib/utils'

const TG_WS_PROXY_PORT_RE = /^\d+$/

function TgWsProxyPageContent({
  port,
  secret,
  tgLink,
  enabled,
  status,
  isBusy,
  applySettings,
  handleToggle,
}: {
  port: number
  secret: string
  tgLink: string
  enabled: boolean
  status: ReturnType<typeof useTgWsProxyModule>['status']
  isBusy: boolean
  applySettings: ReturnType<typeof useTgWsProxyModule>['applySettings']
  handleToggle: ReturnType<typeof useTgWsProxyModule>['handleToggle']
}) {
  const [draftPort, setDraftPort] = useState(String(port))
  const [draftSecret, setDraftSecret] = useState(secret)
  const toggleButtonRef = useRef<HTMLButtonElement | null>(null)
  const tgHttpLink = buildTgWsProxyHttpLink(port, secret)

  const applyDraftSettings = async (nextDraftPort = draftPort, nextDraftSecret = draftSecret) => {
    const normalizedSecret = normalizeTgWsProxySecret(nextDraftSecret)
    const isPortNumeric = TG_WS_PROXY_PORT_RE.test(nextDraftPort)
    const parsedPort = isPortNumeric ? Number.parseInt(nextDraftPort, 10) : Number.NaN

    if (parsedPort === port && normalizedSecret === secret) {
      return true
    }

    return applySettings(parsedPort, normalizedSecret)
  }

  const handleDraftBlur = async (event: React.FocusEvent<HTMLInputElement>) => {
    if (toggleButtonRef.current?.contains(event.relatedTarget as Node | null)) {
      return
    }

    await applyDraftSettings()
  }

  const handleToggleWithDraftSync = async () => {
    if (!enabled) {
      const applied = await applyDraftSettings()
      if (!applied) {
        return
      }
    }

    await handleToggle()
  }

  const handleCopyLink = async () => {
    try {
      await navigator.clipboard.writeText(tgLink)
      toast.success('Ссылка TG WS Proxy скопирована')
    }
    catch (error) {
      toast.error(`Не удалось скопировать ссылку: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  const handleOpenTelegram = async () => {
    try {
      await openUrl(tgLink)
    }
    catch {
      try {
        await openUrl(tgHttpLink)
      }
      catch (fallbackError) {
        const reason = fallbackError instanceof Error ? fallbackError.message : String(fallbackError)
        toast.error(`Не удалось открыть Telegram-ссылку: ${reason}`)
      }
    }
  }

  return (
    <LenisScrollArea className="h-full min-h-0">
      <div className="space-y-6 p-6">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex items-center gap-4">
            <Link to="/modules" className="cursor-pointer text-muted-foreground hover:text-foreground" aria-label="Назад к модулям">
              <ArrowLeft className="size-5" />
            </Link>
            <div>
              <h1 className="text-2xl font-medium">TG WS Proxy</h1>
              <p className="mt-1 text-sm text-muted-foreground">
                Локальный MTProto-прокси для Telegram Desktop через WebSocket
              </p>
            </div>
          </div>
          <Button
            ref={toggleButtonRef}
            type="button"
            variant={enabled ? 'destructive' : 'default'}
            className={cn('gap-2', enabled && 'shadow-none hover:shadow-none')}
            disabled={isBusy || status == null || (!enabled && status.moduleAvailable === false)}
            onClick={() => { void handleToggleWithDraftSync() }}
          >
            {isBusy
              ? <Loader2 className="size-4 animate-spin" />
              : <Power className="size-4" />}
            {isBusy
              ? 'Сохранение...'
              : enabled ? 'Выключить модуль' : 'Включить модуль'}
          </Button>
        </div>

        <Card className={MODULE_PAGE_CARD_CLASS}>
          <ModuleSectionHeader
            icon={ShieldCheck}
            title="Параметры"
            description="Основные параметры локального Telegram-прокси на этом ПК"
          />
          <CardContent className="space-y-4 p-4!">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
              <ModuleSettingLabel
                htmlFor="tg-ws-proxy-port"
                icon={Link2}
                description="Локальный порт, через который Telegram Desktop подключается к прокси."
              >
                Порт
              </ModuleSettingLabel>
              <div className="w-full sm:w-[8rem]">
                <Input
                  id="tg-ws-proxy-port"
                  inputMode="numeric"
                  value={draftPort}
                  disabled={isBusy}
                  onChange={event => setDraftPort(event.target.value)}
                  onBlur={(event) => { void handleDraftBlur(event) }}
                />
              </div>
            </div>

            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
              <ModuleSettingLabel
                htmlFor="tg-ws-proxy-secret"
                icon={KeyRound}
                description="32 шестнадцатеричных символа."
              >
                Секрет
              </ModuleSettingLabel>
              <div className="relative w-full sm:w-[24rem]">
                <Input
                  id="tg-ws-proxy-secret"
                  className="pr-10"
                  value={draftSecret}
                  disabled={isBusy}
                  onChange={event => setDraftSecret(event.target.value)}
                  onBlur={(event) => { void handleDraftBlur(event) }}
                />
                <button
                  type="button"
                  className="text-muted-foreground hover:text-foreground absolute top-1/2 right-3 inline-flex size-4 -translate-y-1/2 cursor-pointer items-center justify-center transition-colors disabled:cursor-not-allowed disabled:opacity-50"
                  disabled={isBusy}
                  aria-label="Сгенерировать новый секрет"
                  onMouseDown={event => event.preventDefault()}
                  onClick={() => {
                    const nextSecret = generateTgWsProxySecret()
                    setDraftSecret(nextSecret)
                    void applyDraftSettings(draftPort, nextSecret)
                  }}
                >
                  <RefreshCw className="size-4" />
                </button>
              </div>
            </div>

          </CardContent>
        </Card>

        <Card className={MODULE_PAGE_CARD_CLASS}>
          <ModuleSectionHeader
            icon={Send}
            title="Подключение"
            description="Используйте ссылку ниже, чтобы быстро добавить прокси в Telegram Desktop"
            action={(
              <Button type="button" className="gap-2 shadow-none hover:shadow-none" onClick={() => void handleOpenTelegram()}>
                <Link2 className="size-4" />
                Открыть в Telegram
              </Button>
            )}
          />
          <CardContent className="space-y-4 p-4!">
            <div className="grid gap-3 sm:grid-cols-3">
              <div className="rounded-xl border border-border/60 bg-muted/18 p-3">
                <p className="text-xs text-muted-foreground">Хост</p>
                <div className="-mx-3 mt-2 border-t border-border/60 px-3 pt-2">
                  <div className="flex items-end justify-between gap-3">
                    <p className="text-sm font-medium">127.0.0.1</p>
                    <button
                      type="button"
                      className="text-muted-foreground hover:text-foreground inline-flex size-4 shrink-0 cursor-pointer items-center justify-center transition-colors"
                      aria-label="Скопировать хост"
                      onClick={() => void navigator.clipboard.writeText('127.0.0.1').then(() => toast.success('Хост скопирован')).catch((error) => {
                        toast.error(`Не удалось скопировать хост: ${error instanceof Error ? error.message : String(error)}`)
                      })}
                    >
                      <Copy className="size-4" />
                    </button>
                  </div>
                </div>
              </div>
              <div className="rounded-xl border border-border/60 bg-muted/18 p-3">
                <p className="text-xs text-muted-foreground">Порт</p>
                <div className="-mx-3 mt-2 border-t border-border/60 px-3 pt-2">
                  <div className="flex items-end justify-between gap-3">
                    <p className="text-sm font-medium">{port}</p>
                    <button
                      type="button"
                      className="text-muted-foreground hover:text-foreground inline-flex size-4 shrink-0 cursor-pointer items-center justify-center transition-colors"
                      aria-label="Скопировать порт"
                      onClick={() => void navigator.clipboard.writeText(String(port)).then(() => toast.success('Порт скопирован')).catch((error) => {
                        toast.error(`Не удалось скопировать порт: ${error instanceof Error ? error.message : String(error)}`)
                      })}
                    >
                      <Copy className="size-4" />
                    </button>
                  </div>
                </div>
              </div>
              <div className="rounded-xl border border-border/60 bg-muted/18 p-3">
                <p className="text-xs text-muted-foreground">PID</p>
                <div className="-mx-3 mt-2 border-t border-border/60 px-3 pt-2">
                  <div className="flex items-end justify-between gap-3">
                    <p className="text-sm font-medium">{status?.pid ?? 'не запущен'}</p>
                    <button
                      type="button"
                      className="text-muted-foreground hover:text-foreground inline-flex size-4 shrink-0 cursor-pointer items-center justify-center transition-colors"
                      aria-label="Скопировать PID"
                      onClick={() => void navigator.clipboard.writeText(String(status?.pid ?? 'не запущен')).then(() => toast.success('PID скопирован')).catch((error) => {
                        toast.error(`Не удалось скопировать PID: ${error instanceof Error ? error.message : String(error)}`)
                      })}
                    >
                      <Copy className="size-4" />
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <div className="rounded-xl border border-border/60 bg-muted/18 p-3">
              <p className="text-xs text-muted-foreground">Ссылка</p>
              <div className="-mx-3 mt-2 border-t border-border/60 px-3 pt-2">
                <div className="flex items-end justify-between gap-3">
                  <p className="break-all text-sm">{tgLink}</p>
                  <button
                    type="button"
                    className="text-muted-foreground hover:text-foreground inline-flex size-4 shrink-0 cursor-pointer items-center justify-center transition-colors"
                    aria-label="Скопировать ссылку TG WS Proxy"
                    onClick={() => void handleCopyLink()}
                  >
                    <Copy className="size-4" />
                  </button>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </LenisScrollArea>
  )
}

export function TgWsProxyPage() {
  const {
    config,
    loading,
    status,
    isBusy,
    port,
    secret,
    tgLink,
    enabled,
    applySettings,
    handleToggle,
  } = useTgWsProxyModule()

  if (loading || !config) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="size-6 animate-spin" />
      </div>
    )
  }

  return (
    <TgWsProxyPageContent
      key={`${port}:${secret}`}
      port={port}
      secret={secret}
      tgLink={tgLink}
      enabled={enabled}
      status={status}
      isBusy={isBusy}
      applySettings={applySettings}
      handleToggle={handleToggle}
    />
  )
}
