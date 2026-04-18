import type { ReactNode } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { ChevronRight, Globe, Loader2, Send } from 'lucide-react'
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import { Switch } from '@/components/ui/switch'
import { useDnsModuleSummary, useTgWsProxyModuleSummary } from '@/hooks/use-module-summary'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { cn } from '@/lib/utils'
import { useConfigStore } from '@/stores/config.store'

function ModuleCard({
  title,
  description,
  icon,
  enabled,
  status,
  isBusy,
  openModule,
  handleToggle,
}: {
  title: string
  description: string
  icon: ReactNode
  enabled: boolean
  status: { running: boolean, moduleAvailable: boolean } | null
  isBusy: boolean
  openModule: () => void
  handleToggle: () => void
}) {
  return (
    <div
      className={cn(
        'group flex h-20 items-center gap-3 rounded-lg border bg-card p-4',
        status?.moduleAvailable === false && 'opacity-60',
      )}
    >
      <div
        className="flex min-w-0 flex-1 cursor-pointer items-center gap-3"
        role="link"
        tabIndex={0}
        aria-label={`Открыть модуль ${title}`}
        onClick={openModule}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault()
            openModule()
          }
        }}
      >
        <div className="text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-muted/25">
          {icon}
        </div>
        <div className="min-w-0 flex-1">
          <span className="block truncate text-sm font-normal">{title}</span>
          <p className="mt-1 text-xs text-muted-foreground">
            {description}
          </p>
        </div>
        <span className="text-muted-foreground flex size-5 shrink-0 items-center justify-center">
          <ChevronRight className="size-4 transition-transform group-hover:translate-x-1" />
        </span>
      </div>

      <div
        className="flex shrink-0 items-center"
        onClick={event => event.stopPropagation()}
        onKeyDown={event => event.stopPropagation()}
      >
        <Switch
          size="sm"
          checked={enabled}
          aria-label={`Переключить модуль ${title}`}
          disabled={isBusy || status == null || status.moduleAvailable === false}
          onCheckedChange={handleToggle}
        />
      </div>
    </div>
  )
}

export function ModulesPage() {
  const navigate = useNavigate()
  const config = useConfigStore(state => state.config)
  const loading = useConfigStore(state => state.loading)
  const load = useConfigStore(state => state.load)
  const dnsModule = useDnsModuleSummary()
  const tgWsProxyModule = useTgWsProxyModuleSummary()

  useMountEffect(() => {
    void load()
  })

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
          <h1 className="text-2xl font-medium">Модули</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Дополнительные инструменты приложения
          </p>
        </div>

        <div className="space-y-3">
          <ModuleCard
            title="DNS"
            description="Дополнительный обход геоблока иностранных сервисов через DNS"
            icon={<Globe className="size-4" />}
            enabled={dnsModule.enabled}
            status={dnsModule.status
              ? {
                  running: dnsModule.status.running,
                  moduleAvailable: dnsModule.status.moduleAvailable,
                }
              : null}
            isBusy={dnsModule.isBusy}
            openModule={() => {
              void navigate({ to: '/modules/dns' })
            }}
            handleToggle={() => {
              void dnsModule.handleToggle()
            }}
          />

          <ModuleCard
            title="TG WS Proxy"
            description="Локальный MTProto-прокси для Telegram Desktop через WebSocket"
            icon={<Send className="size-4" />}
            enabled={tgWsProxyModule.enabled}
            status={tgWsProxyModule.status
              ? {
                  running: tgWsProxyModule.status.running,
                  moduleAvailable: tgWsProxyModule.status.moduleAvailable,
                }
              : null}
            isBusy={tgWsProxyModule.isBusy}
            openModule={() => {
              void navigate({ to: '/modules/tg-ws-proxy' })
            }}
            handleToggle={() => {
              void tgWsProxyModule.handleToggle()
            }}
          />
        </div>
      </div>
    </LenisScrollArea>
  )
}
