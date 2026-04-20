import { openUrl } from '@tauri-apps/plugin-opener'
import { Copy, KeyRound, Link2, Loader2, Power, RefreshCw, Send, ShieldCheck } from 'lucide-react'
import { useState } from 'react'
import { toast } from 'sonner'
import { MODULE_PAGE_CARD_CLASS, ModuleSectionHeader, ModuleSettingLabel } from '@/components/features/module-ui'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import { useTgWsProxyModule } from '@/hooks/use-tg-ws-proxy-module'
import { buildTgWsProxyHttpLink, generateTgWsProxySecret, isValidTgWsProxySecret, normalizeTgWsProxySecret } from '@/lib/tg-ws-proxy'
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
  const tgHttpLink = buildTgWsProxyHttpLink(port, secret)

  const normalizedDraftSecret = normalizeTgWsProxySecret(draftSecret)
  const isDraftPortNumeric = TG_WS_PROXY_PORT_RE.test(draftPort)
  const parsedDraftPort = isDraftPortNumeric ? Number.parseInt(draftPort, 10) : Number.NaN
  const canApply = parsedDraftPort === port && normalizedDraftSecret === secret
    ? false
    : Number.isInteger(parsedDraftPort) && parsedDraftPort >= 1 && parsedDraftPort <= 65535 && isValidTgWsProxySecret(normalizedDraftSecret)

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
          <div>
            <h1 className="text-2xl font-medium">TG WS Proxy</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              Локальный MTProto-прокси для Telegram Desktop через WebSocket
            </p>
          </div>
          <Button
            type="button"
            variant={enabled ? 'destructive' : 'default'}
            className={cn('gap-2', enabled && 'shadow-none hover:shadow-none')}
            disabled={isBusy || status == null || (!enabled && status.moduleAvailable === false)}
            onClick={() => void handleToggle()}
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
            <div className="space-y-2">
              <ModuleSettingLabel
                htmlFor="tg-ws-proxy-port"
                icon={Link2}
                description="Локальный порт, через который Telegram Desktop подключается к прокси."
              >
                Порт
              </ModuleSettingLabel>
              <Input
                id="tg-ws-proxy-port"
                inputMode="numeric"
                value={draftPort}
                disabled={isBusy}
                onChange={event => setDraftPort(event.target.value)}
              />
            </div>

            <div className="space-y-2">
              <ModuleSettingLabel
                htmlFor="tg-ws-proxy-secret"
                icon={KeyRound}
                description="Секретный ключ для подключения клиента к локальному прокси."
              >
                Секрет
              </ModuleSettingLabel>
              <div className="flex gap-2">
                <Input
                  id="tg-ws-proxy-secret"
                  value={draftSecret}
                  disabled={isBusy}
                  onChange={event => setDraftSecret(event.target.value)}
                />
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  className="shrink-0"
                  disabled={isBusy}
                  aria-label="Сгенерировать новый секрет"
                  onClick={() => setDraftSecret(generateTgWsProxySecret())}
                >
                  <RefreshCw className="size-4" />
                </Button>
              </div>
            </div>

            <div className="flex justify-end">
              <Button
                type="button"
                className="gap-2"
                disabled={isBusy || !canApply}
                onClick={() => void applySettings(parsedDraftPort, normalizedDraftSecret)}
              >
                {isBusy
                  ? <Loader2 className="size-4 animate-spin" />
                  : <KeyRound className="size-4" />}
                Применить
              </Button>
            </div>
          </CardContent>
        </Card>

        <Card className={MODULE_PAGE_CARD_CLASS}>
          <ModuleSectionHeader
            icon={Send}
            title="Подключение"
            description="Используйте ссылку ниже, чтобы быстро добавить прокси в Telegram Desktop"
          />
          <CardContent className="space-y-4 p-4!">
            <div className="grid gap-3 sm:grid-cols-3">
              <div className="rounded-xl border border-border/60 bg-muted/18 p-3">
                <p className="text-xs text-muted-foreground">Хост</p>
                <p className="mt-1 text-sm font-medium">127.0.0.1</p>
              </div>
              <div className="rounded-xl border border-border/60 bg-muted/18 p-3">
                <p className="text-xs text-muted-foreground">Порт</p>
                <p className="mt-1 text-sm font-medium">{port}</p>
              </div>
              <div className="rounded-xl border border-border/60 bg-muted/18 p-3">
                <p className="text-xs text-muted-foreground">PID</p>
                <p className="mt-1 text-sm font-medium">{status?.pid ?? 'не запущен'}</p>
              </div>
            </div>

            <div className="rounded-xl border border-border/60 bg-muted/18 p-3">
              <p className="text-xs text-muted-foreground">Ссылка</p>
              <p className="mt-1 break-all text-sm">{tgLink}</p>
            </div>

            <div className="flex flex-wrap gap-2">
              <Button type="button" variant="outline" className="gap-2" onClick={() => void handleCopyLink()}>
                <Copy className="size-4" />
                Копировать ссылку
              </Button>
              <Button type="button" className="gap-2" onClick={() => void handleOpenTelegram()}>
                <Link2 className="size-4" />
                Открыть в Telegram
              </Button>
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
