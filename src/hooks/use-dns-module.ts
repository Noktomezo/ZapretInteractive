import type { DnsProxyStatus } from '@/lib/types'
import { useState } from 'react'
import { toast } from 'sonner'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { applyDnsAccelerator, DEFAULT_BOOTSTRAP_RESOLVER, DNS_PRESETS, normalizeDnsPresetId } from '@/lib/dns'
import * as tauri from '@/lib/tauri'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'

export function useDnsModule() {
  const [status, setStatus] = useState<DnsProxyStatus | null>(null)
  const [isBusy, setIsBusy] = useState(false)
  const [isCheckingLatency, setIsCheckingLatency] = useState(false)
  const [latencyByPreset, setLatencyByPreset] = useState<Record<string, number | null>>({})

  const config = useConfigStore(state => state.config)
  const loading = useConfigStore(state => state.loading)
  const load = useConfigStore(state => state.load)
  const saveNow = useConfigStore(state => state.saveNow)
  const scheduleSave = useConfigStore(state => state.scheduleSave)
  const setDnsPresetId = useConfigStore(state => state.setDnsPresetId)
  const setDnsBootstrapResolvers = useConfigStore(state => state.setDnsBootstrapResolvers)
  const setDnsAcceleratorEnabled = useConfigStore(state => state.setDnsAcceleratorEnabled)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)

  const selectedPresetId = normalizeDnsPresetId(config?.dnsPresetId)
  const selectedPreset = DNS_PRESETS.find(preset => preset.id === selectedPresetId) ?? DNS_PRESETS[0]
  const selectedBootstrapResolver = config?.dnsBootstrapResolvers?.[0] ?? DEFAULT_BOOTSTRAP_RESOLVER
  const acceleratorEnabled = config?.dnsAcceleratorEnabled ?? false

  const refreshStatus = async () => {
    const nextStatus = await tauri.getDnsProxyStatus()
    setStatus(nextStatus)
    return nextStatus
  }

  const handleCheckLatency = async () => {
    setIsCheckingLatency(true)
    setLatencyByPreset({})
    try {
      const targets = DNS_PRESETS.map(preset => ({ id: preset.id, url: preset.urls[0] }))
      await Promise.allSettled(targets.map(async (target) => {
        const [result] = await tauri.checkDnsProviderLatency([target.url])
        setLatencyByPreset(current => ({
          ...current,
          [target.id]: result?.reachable ? result.latencyMs ?? null : null,
        }))
      }))
      toast.success('Пинг DNS обновлён')
    }
    catch (error) {
      toast.error(`Ошибка проверки пинга DNS: ${error instanceof Error ? error.message : String(error)}`)
    }
    finally {
      setIsCheckingLatency(false)
    }
  }

  const applyDnsChanges = async ({
    presetId,
    bootstrapResolver,
    nextAcceleratorEnabled,
  }: {
    presetId?: string
    bootstrapResolver?: string
    nextAcceleratorEnabled?: boolean
  }) => {
    const nextPresetId = normalizeDnsPresetId(presetId ?? selectedPreset.id)
    const nextPreset = DNS_PRESETS.find(item => item.id === nextPresetId) ?? DNS_PRESETS[0]
    const nextBootstrapResolver = bootstrapResolver ?? selectedBootstrapResolver
    const accelerator = nextAcceleratorEnabled ?? acceleratorEnabled

    setDnsPresetId(nextPreset.id)
    setDnsBootstrapResolvers([nextBootstrapResolver])
    setDnsAcceleratorEnabled(accelerator)

    if (!status?.running) {
      scheduleSave('dns-settings')
      return
    }

    setIsBusy(true)
    try {
      await saveNow()
      const dohUrls = applyDnsAccelerator(nextPreset.urls.slice(), accelerator)
      await tauri.stopDnsProxy()
      const nextStatus = await tauri.startDnsProxy(dohUrls, [nextBootstrapResolver])
      setStatus(nextStatus)
      addConfigLog(`dnscrypt-proxy перезапущен (${nextPreset.name})`)
      toast.success('Настройки DNS применены')
    }
    catch (error) {
      toast.error(`Ошибка применения настроек DNS: ${error instanceof Error ? error.message : String(error)}`)
      await refreshStatus().catch(() => {})
    }
    finally {
      setIsBusy(false)
    }
  }

  useMountEffect(() => {
    let isMounted = true

    const init = async () => {
      await load()
      const nextStatus = await tauri.getDnsProxyStatus()
      if (isMounted) {
        setStatus(nextStatus)
      }
    }

    void init().catch((error) => {
      if (isMounted) {
        toast.error(`Ошибка инициализации DNS страницы: ${error instanceof Error ? error.message : String(error)}`)
      }
    })

    return () => {
      isMounted = false
    }
  })

  const handlePresetSelect = (presetId: string) => {
    void applyDnsChanges({ presetId })
  }

  const handleBootstrapSelect = (resolver: string) => {
    void applyDnsChanges({ bootstrapResolver: resolver })
  }

  const handleAcceleratorChange = (checked: boolean) => {
    void applyDnsChanges({ nextAcceleratorEnabled: checked })
  }

  const handleToggle = async () => {
    if (!config) {
      return
    }

    const normalizedResolvers = [selectedBootstrapResolver]
    setDnsBootstrapResolvers(normalizedResolvers)
    setDnsPresetId(selectedPreset.id)

    setIsBusy(true)
    try {
      await saveNow()
      const dohUrls = applyDnsAccelerator(selectedPreset.urls.slice(), acceleratorEnabled)

      const nextStatus = status?.running
        ? await tauri.stopDnsProxy()
        : await tauri.startDnsProxy(dohUrls, normalizedResolvers)

      setStatus(nextStatus)
      addConfigLog(status?.running
        ? 'dnscrypt-proxy отключён и системный DNS восстановлен'
        : `dnscrypt-proxy включён (${selectedPreset.name})`)
      toast.success(status?.running ? 'DNS выключён' : 'DNS включён')
    }
    catch (error) {
      toast.error(`Ошибка переключения DNS: ${error instanceof Error ? error.message : String(error)}`)
      await refreshStatus().catch(() => {})
    }
    finally {
      setIsBusy(false)
    }
  }

  return {
    config,
    loading,
    status,
    isBusy,
    isCheckingLatency,
    latencyByPreset,
    selectedPreset,
    selectedBootstrapResolver,
    acceleratorEnabled,
    handleCheckLatency,
    handlePresetSelect,
    handleBootstrapSelect,
    handleAcceleratorChange,
    handleToggle,
  }
}
