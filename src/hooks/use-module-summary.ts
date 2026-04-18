import type { DnsProxyStatus, TgWsProxyStatus } from '@/lib/types'
import { useState } from 'react'
import { toast } from 'sonner'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { applyDnsAccelerator, DEFAULT_BOOTSTRAP_RESOLVER, DNS_PRESETS, normalizeDnsPresetId } from '@/lib/dns'
import * as tauri from '@/lib/tauri'
import { DEFAULT_TG_WS_PROXY_PORT, isValidTgWsProxySecret, normalizeTgWsProxySecret } from '@/lib/tg-ws-proxy'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'

export function useDnsModuleSummary() {
  const [status, setStatus] = useState<DnsProxyStatus | null>(null)
  const [isBusy, setIsBusy] = useState(false)

  const config = useConfigStore(state => state.config)
  const saveNow = useConfigStore(state => state.saveNow)
  const setDnsModuleEnabled = useConfigStore(state => state.setDnsModuleEnabled)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)
  const connectionStatus = useConnectionStore(state => state.status)

  const selectedPresetId = normalizeDnsPresetId(config?.dnsPresetId)
  const selectedPreset = DNS_PRESETS.find(preset => preset.id === selectedPresetId) ?? DNS_PRESETS[0]
  const selectedBootstrapResolver = config?.dnsBootstrapResolvers?.[0] ?? DEFAULT_BOOTSTRAP_RESOLVER
  const acceleratorEnabled = config?.dnsAcceleratorEnabled ?? false
  const enabled = config?.dnsModuleEnabled ?? false

  const refreshStatus = async () => {
    const nextStatus = await tauri.getDnsProxyStatus()
    setStatus(nextStatus)
    return nextStatus
  }

  const resolveStatus = async () => status ?? refreshStatus()

  const handleToggle = async () => {
    if (!config) {
      return
    }

    const previousEnabled = enabled
    const nextEnabled = !enabled
    setDnsModuleEnabled(nextEnabled)

    let currentStatus: DnsProxyStatus
    try {
      currentStatus = await resolveStatus()
      setIsBusy(true)
      await saveNow()
    }
    catch (error) {
      setDnsModuleEnabled(previousEnabled)
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
            [selectedBootstrapResolver],
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

  useMountEffect(() => {
    let isMounted = true

    void tauri.getDnsProxyStatus()
      .then((nextStatus) => {
        if (isMounted) {
          setStatus(nextStatus)
        }
      })
      .catch((error) => {
        if (isMounted) {
          toast.error(`Ошибка инициализации DNS страницы: ${error instanceof Error ? error.message : String(error)}`)
        }
      })

    return () => {
      isMounted = false
    }
  })

  return {
    status,
    enabled,
    isBusy,
    handleToggle,
  }
}

export function useTgWsProxyModuleSummary() {
  const [status, setStatus] = useState<TgWsProxyStatus | null>(null)
  const [isBusy, setIsBusy] = useState(false)

  const config = useConfigStore(state => state.config)
  const saveNow = useConfigStore(state => state.saveNow)
  const revertTo = useConfigStore(state => state.revertTo)
  const setTgWsProxyModuleEnabled = useConfigStore(state => state.setTgWsProxyModuleEnabled)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)
  const connectionStatus = useConnectionStore(state => state.status)

  const port = config?.tgWsProxyPort ?? DEFAULT_TG_WS_PROXY_PORT
  const secret = normalizeTgWsProxySecret(config?.tgWsProxySecret)
  const enabled = config?.tgWsProxyModuleEnabled ?? false

  const refreshStatus = async () => {
    const nextStatus = await tauri.getTgWsProxyStatus()
    setStatus(nextStatus)
    return nextStatus
  }

  const resolveStatus = async () => status ?? refreshStatus()

  const handleToggle = async () => {
    if (!config) {
      return
    }

    const previousConfig = structuredClone(config)
    const nextEnabled = !enabled
    if (nextEnabled && !isValidTgWsProxySecret(secret)) {
      toast.error('Сначала сохраните корректный секрет TG WS Proxy')
      return
    }

    setTgWsProxyModuleEnabled(nextEnabled)

    try {
      await resolveStatus()
      setIsBusy(true)
      await saveNow()
    }
    catch (error) {
      revertTo(previousConfig)
      toast.error(`Ошибка переключения TG WS Proxy: ${error instanceof Error ? error.message : String(error)}`)
      await refreshStatus().catch(() => {})
      setIsBusy(false)
      return
    }

    try {
      if (connectionStatus === 'connected') {
        const currentStatus = await resolveStatus()
        const nextStatus = nextEnabled && !currentStatus.running
          ? await tauri.startTgWsProxy(port, secret)
          : !nextEnabled && currentStatus.running
              ? await tauri.stopTgWsProxy()
              : currentStatus

        setStatus(nextStatus)
        addConfigLog(nextEnabled
          ? `TG WS Proxy модуль включён и запущен на порту ${port}`
          : 'TG WS Proxy модуль выключен и остановлен')
      }
      else {
        await refreshStatus().catch(() => {})
        addConfigLog(nextEnabled ? 'TG WS Proxy модуль включён' : 'TG WS Proxy модуль выключен')
      }

      toast.success(
        connectionStatus === 'connected'
          ? nextEnabled ? 'TG WS Proxy модуль включён' : 'TG WS Proxy модуль выключен'
          : nextEnabled ? 'TG WS Proxy модуль включён и будет запущен при подключении' : 'TG WS Proxy модуль выключен',
      )
    }
    catch (error) {
      toast.error(`Ошибка переключения TG WS Proxy: ${error instanceof Error ? error.message : String(error)}`)
      await refreshStatus().catch(() => {})
    }
    finally {
      setIsBusy(false)
    }
  }

  useMountEffect(() => {
    let isMounted = true

    void tauri.getTgWsProxyStatus()
      .then((nextStatus) => {
        if (isMounted) {
          setStatus(nextStatus)
        }
      })
      .catch((error) => {
        if (isMounted) {
          toast.error(`Ошибка инициализации TG WS Proxy: ${error instanceof Error ? error.message : String(error)}`)
        }
      })

    return () => {
      isMounted = false
    }
  })

  return {
    status,
    enabled,
    isBusy,
    handleToggle,
  }
}
