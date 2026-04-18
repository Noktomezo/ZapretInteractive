import {
  Check,
  Globe,
  Loader2,
  Power,
  RefreshCw,
  ShieldCheck,
} from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
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

const PAGE_CARD_CLASS = '!border-border/60 !bg-background !shadow-none !backdrop-blur-none'

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
            disabled={isBusy || status == null || status.moduleAvailable === false}
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
          <CardHeader className="gap-1">
            <CardTitle className="flex items-center gap-2 text-lg">
              <ShieldCheck className="size-4 text-muted-foreground" />
              <span>Параметры</span>
            </CardTitle>
            <CardDescription>
              Основные параметры работы DNS-подключения
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
              <div className="space-y-0.5">
                <Label htmlFor="dns-bootstrap-resolvers">Начальный резолвер</Label>
                <p className="text-xs text-muted-foreground">
                  Нужен для первого подключения к DNS-серверу.
                </p>
              </div>
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
              <div className="space-y-0.5">
                <Label htmlFor="dns-accelerator">Акселератор</Label>
                <p className="text-xs text-muted-foreground">
                  Может ускорить работу DNS и сделать подключение к некоторым сервисам стабильнее.
                </p>
              </div>
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
          <CardHeader className="gap-1">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <div className="space-y-1">
                <CardTitle className="flex items-center gap-2 text-lg">
                  <Globe className="size-4 text-muted-foreground" />
                  <span>DNS провайдеры</span>
                </CardTitle>
                <CardDescription>
                  Выбор адреса для DNS-подключения
                </CardDescription>
              </div>
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
            </div>
          </CardHeader>
          <CardContent className="grid gap-3 lg:grid-cols-2">
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
