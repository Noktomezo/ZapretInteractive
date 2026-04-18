import { useNavigate } from '@tanstack/react-router'
import { ChevronRight, Globe, Loader2 } from 'lucide-react'
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import { Switch } from '@/components/ui/switch'
import { useDnsModule } from '@/hooks/use-dns-module'
import { cn } from '@/lib/utils'

export function ModulesPage() {
  const navigate = useNavigate()
  const { config, loading, status, isBusy, handleToggle } = useDnsModule()

  const openDnsPage = () => {
    void navigate({ to: '/modules/dns' })
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
          <h1 className="text-2xl font-medium">Модули</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Дополнительные инструменты приложения
          </p>
        </div>

        <div
          className={cn(
            'group flex h-20 cursor-pointer items-center gap-3 rounded-lg border bg-card p-4',
            status?.moduleAvailable === false && 'opacity-60',
          )}
        >
          <div
            className="flex min-w-0 flex-1 items-center gap-3"
            role="link"
            tabIndex={0}
            aria-label="Открыть модуль DNS"
            onClick={openDnsPage}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault()
                openDnsPage()
              }
            }}
          >
            <div className="text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-muted/25">
              <Globe className="size-4" />
            </div>
            <div className="min-w-0 flex-1">
              <span className="block truncate text-sm font-normal">DNS</span>
              <p className="mt-1 text-xs text-muted-foreground">
                Дополнительный обход геоблока иностранных сервисов через DNS
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
              checked={Boolean(status?.running)}
              aria-label="Переключить DNS модуль"
              disabled={isBusy || status == null || status.moduleAvailable === false}
              onCheckedChange={() => void handleToggle()}
            />
          </div>
        </div>
      </div>
    </LenisScrollArea>
  )
}
