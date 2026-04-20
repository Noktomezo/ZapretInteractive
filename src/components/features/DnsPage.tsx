import {
  Check,
  Globe,
  Loader2,
  Power,
  RefreshCw,
  Route,
  ShieldCheck,
} from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
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
import { useDnsModule } from '@/hooks/use-dns-module'
import { BOOTSTRAP_RESOLVER_OPTIONS, DNS_PRESETS, getDnsLatencyBadgeClass } from '@/lib/dns'
import { cn } from '@/lib/utils'

const PAGE_CARD_CLASS = '!gap-0 !rounded-lg !border !border-border/60 !bg-card !py-0 !shadow-none !backdrop-blur-none'

function ModuleSectionHeader({
  icon: Icon,
  title,
  description,
  action,
}: {
  icon: React.ComponentType<{ className?: string }>
  title: React.ReactNode
  description: React.ReactNode
  action?: React.ReactNode
}) {
  return (
    <CardHeader className="!flex !flex-row !items-center !gap-3 border-b border-border/60 !p-4">
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

function ModuleSettingLabel({
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

export function DnsPage() {
  const {
    config,
    loading,
    status,
    isBusy,
    isCheckingLatency,
    latencyByPreset,
    selectedPreset,
    selectedBootstrapResolver,
    acceleratorEnabled,
    enabled,
    handleCheckLatency,
    handlePresetSelect,
    handleBootstrapSelect,
    handleAcceleratorChange,
    handleToggle,
  } = useDnsModule()

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
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-medium">DNS</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              Дополнительный обход геоблока иностранных сервисов: ИИ, игр, сайтов и других ресурсов
            </p>
          </div>
          <Button
            type="button"
            variant={enabled ? 'destructive' : 'default'}
            className={cn(
              'gap-2',
              enabled && 'shadow-none hover:shadow-none',
            )}
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

        <Card className={PAGE_CARD_CLASS}>
          <ModuleSectionHeader
            icon={ShieldCheck}
            title="Параметры"
            description="Основные параметры работы DNS-подключения"
          />
          <CardContent className="space-y-4 !p-4">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
              <ModuleSettingLabel
                htmlFor="dns-bootstrap-resolvers"
                icon={ShieldCheck}
                description="Нужен для первого подключения к DNS-серверу."
              >
                Начальный резолвер
              </ModuleSettingLabel>
              <div className="w-full sm:w-[11rem]">
                <Select value={selectedBootstrapResolver} onValueChange={handleBootstrapSelect} disabled={isBusy}>
                  <SelectTrigger id="dns-bootstrap-resolvers" className="w-full cursor-pointer">
                    <SelectValue placeholder="Выберите резолвер" />
                  </SelectTrigger>
                  <SelectContent>
                    {BOOTSTRAP_RESOLVER_OPTIONS.map(option => (
                      <SelectItem key={option.value} value={option.value}>
                        <span>{option.label}</span>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>

            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
              <ModuleSettingLabel
                htmlFor="dns-accelerator"
                icon={Route}
                description="Может ускорить работу DNS и сделать подключение к некоторым сервисам стабильнее."
              >
                Акселератор
              </ModuleSettingLabel>
              <Switch
                id="dns-accelerator"
                checked={acceleratorEnabled}
                onCheckedChange={handleAcceleratorChange}
                disabled={isBusy}
              />
            </div>
          </CardContent>
        </Card>

        <Card className={PAGE_CARD_CLASS}>
          <ModuleSectionHeader
            icon={Globe}
            title="DNS провайдеры"
            description="Выбор адреса для DNS-подключения"
            action={(
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="gap-2"
                disabled={isBusy || isCheckingLatency}
                onClick={() => void handleCheckLatency()}
              >
                {isCheckingLatency
                  ? <Loader2 className="size-4 animate-spin" />
                  : <RefreshCw className="size-4" />}
                Проверить пинг
              </Button>
            )}
          />
          <CardContent className="grid gap-3 !p-4 lg:grid-cols-2">
            {DNS_PRESETS.map((preset) => {
              const isSelected = preset.id === selectedPreset.id
              const latency = latencyByPreset[preset.id]
              return (
                <button
                  key={preset.id}
                  type="button"
                  className={cn(
                    'rounded-xl border p-4 text-left transition-colors',
                    isSelected
                      ? 'border-success/50 bg-success/8'
                      : 'border-border/60 bg-muted/18 hover:border-border hover:bg-muted/26',
                    isBusy && 'cursor-not-allowed opacity-60 hover:border-inherit hover:bg-inherit',
                  )}
                  onClick={() => handlePresetSelect(preset.id)}
                  disabled={isBusy}
                >
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <div className="flex items-center gap-2">
                        <p className="text-sm font-medium">{preset.name}</p>
                        {latency !== undefined && (
                          <Badge
                            variant="outline"
                            className={cn(
                              'rounded-full border px-1.5 py-0 text-[10px]',
                              getDnsLatencyBadgeClass(latency),
                            )}
                          >
                            {latency === null ? 'н/д' : `${latency} мс`}
                          </Badge>
                        )}
                      </div>
                      <p className="text-muted-foreground mt-1 break-all text-xs">
                        {preset.urls[0]}
                      </p>
                    </div>
                    <span className="flex size-5 shrink-0 items-center justify-center">
                      {isSelected ? <Check className="text-success size-4" /> : null}
                    </span>
                  </div>
                  {preset.urls.length > 1 && (
                    <div className="mt-3 space-y-1">
                      {preset.urls.slice(1).map(url => (
                        <p key={url} className="text-muted-foreground break-all text-xs">
                          {url}
                        </p>
                      ))}
                    </div>
                  )}
                </button>
              )
            })}
          </CardContent>
        </Card>
      </div>
    </LenisScrollArea>
  )
}
