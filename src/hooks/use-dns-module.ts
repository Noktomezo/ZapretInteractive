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
  const setDnsPresetId = useConfigStore(state => state.setDnsPresetId)
  const setDnsBootstrapResolvers = useConfigStore(state => state.setDnsBootstrapResolvers)
  const setDnsAcceleratorEnabled = useConfigStore(state => state.setDnsAcceleratorEnabled)
  const setDnsModuleEnabled = useConfigStore(state => state.setDnsModuleEnabled)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)
  const connectionStatus = useConnectionStore(state => state.status)

  const selectedPresetId = normalizeDnsPresetId(config?.dnsPresetId)
  const selectedPreset = DNS_PRESETS.find(preset => preset.id === selectedPresetId) ?? DNS_PRESETS[0]
  const selectedBootstrapResolver = config?.dnsBootstrapResolvers?.[0] ?? DEFAULT_BOOTSTRAP_RESOLVER
  const acceleratorEnabled = config?.dnsAcceleratorEnabled ?? false
  const enabled = config?.dnsModuleEnabled ?? false

  const applyDnsState = (presetId: string, bootstrapResolver: string, accelerator: boolean, nextEnabled = enabled) => {
    setDnsPresetId(presetId)
    setDnsBootstrapResolvers([bootstrapResolver])
    setDnsAcceleratorEnabled(accelerator)
    setDnsModuleEnabled(nextEnabled)
  }

  const refreshStatus = async () => {
    const nextStatus = await tauri.getDnsProxyStatus()
    setStatus(nextStatus)
    return nextStatus
  }

  const resolveStatus = async () => status ?? refreshStatus()

  const handleCheckLatency = async () => {
    setIsCheckingLatency(true)
    setLatencyByPreset({})
    try {
      const targets = DNS_PRESETS.map(preset => ({ id: preset.id, url: preset.urls[0] }))
      const settled = await Promise.allSettled(targets.map(async (target) => {
        const [result] = await tauri.checkDnsProviderLatency([target.url])
        setLatencyByPreset(current => ({
          ...current,
          [target.id]: result?.reachable ? result.latencyMs ?? null : null,
        }))

        return result
      }))
      const hasReachableResult = settled.some((entry) => {
        if (entry.status !== 'fulfilled') {
          return false
        }

        return entry.value?.reachable && entry.value.latencyMs !== null
      })

      if (hasReachableResult) {
        toast.success('Пинг DNS обновлён')
      }
      else {
        toast.error('Не удалось измерить пинг DNS')
      }
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
    const previousState = {
      presetId: selectedPreset.id,
      bootstrapResolver: selectedBootstrapResolver,
      acceleratorEnabled,
      enabled,
    }

    applyDnsState(nextPreset.id, nextBootstrapResolver, accelerator, enabled)

    let currentStatus: DnsProxyStatus
    try {
      currentStatus = await resolveStatus()
      setIsBusy(true)
      await saveNow()
    }
    catch (error) {
      applyDnsState(
        previousState.presetId,
        previousState.bootstrapResolver,
        previousState.acceleratorEnabled,
        previousState.enabled,
      )
      toast.error(`Ошибка применения настроек DNS: ${error instanceof Error ? error.message : String(error)}`)
      await refreshStatus().catch(() => {})
      setIsBusy(false)
      return
    }

    if (!enabled || connectionStatus !== 'connected') {
      setIsBusy(false)
      toast.success('Параметры DNS сохранены')
      return
    }

    try {
      const dohUrls = applyDnsAccelerator(nextPreset.urls.slice(), accelerator)
      if (currentStatus.running) {
        await tauri.stopDnsProxy()
      }
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

    const nextEnabled = !enabled
    applyDnsState(selectedPreset.id, selectedBootstrapResolver, acceleratorEnabled, nextEnabled)

    const normalizedResolvers = [selectedBootstrapResolver]
    const previousState = {
      presetId: selectedPreset.id,
      bootstrapResolver: selectedBootstrapResolver,
      acceleratorEnabled,
      enabled,
    }

    let currentStatus: DnsProxyStatus
    try {
      currentStatus = await resolveStatus()
      setIsBusy(true)
      await saveNow()
    }
    catch (error) {
      applyDnsState(
        previousState.presetId,
        previousState.bootstrapResolver,
        previousState.acceleratorEnabled,
        previousState.enabled,
      )
      toast.error(`Ошибка переключения DNS: ${error instanceof Error ? error.message : String(error)}`)
      await refreshStatus().catch(() => {})
      setIsBusy(false)
      return
    }

    try {
      if (connectionStatus === 'connected') {
        let nextStatus = currentStatus
        if (nextEnabled && !currentStatus.running) {
          nextStatus = await tauri.startDnsProxy(
            applyDnsAccelerator(selectedPreset.urls.slice(), acceleratorEnabled),
            normalizedResolvers,
          )
        }
        else if (!nextEnabled && currentStatus.running) {
          nextStatus = await tauri.stopDnsProxy()
        }

        setStatus(nextStatus)
        addConfigLog(nextEnabled
          ? `DNS модуль включён и запущен (${selectedPreset.name})`
          : 'DNS модуль выключен и остановлен')
      }
      else {
        await refreshStatus().catch(() => {})
        addConfigLog(nextEnabled ? 'DNS модуль включён' : 'DNS модуль выключен')
      }

      toast.success(
        connectionStatus === 'connected'
          ? nextEnabled ? 'DNS модуль включён' : 'DNS модуль выключен'
          : nextEnabled ? 'DNS модуль включён и будет запущен при подключении' : 'DNS модуль выключен',
      )
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
    enabled,
    handleCheckLatency,
    handlePresetSelect,
    handleBootstrapSelect,
    handleAcceleratorChange,
    handleToggle,
  }
}
