import type { TgWsProxyStatus } from '@/lib/types'
import { useState } from 'react'
import { toast } from 'sonner'
import { useMountEffect } from '@/hooks/use-mount-effect'
import * as tauri from '@/lib/tauri'
import { buildTgWsProxyLink, DEFAULT_TG_WS_PROXY_PORT, isValidTgWsProxySecret, normalizeTgWsProxySecret } from '@/lib/tg-ws-proxy'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'

export function useTgWsProxyModule() {
  const [status, setStatus] = useState<TgWsProxyStatus | null>(null)
  const [isBusy, setIsBusy] = useState(false)

  const config = useConfigStore(state => state.config)
  const loading = useConfigStore(state => state.loading)
  const load = useConfigStore(state => state.load)
  const saveNow = useConfigStore(state => state.saveNow)
  const revertTo = useConfigStore(state => state.revertTo)
  const setTgWsProxyPort = useConfigStore(state => state.setTgWsProxyPort)
  const setTgWsProxySecret = useConfigStore(state => state.setTgWsProxySecret)
  const setTgWsProxyModuleEnabled = useConfigStore(state => state.setTgWsProxyModuleEnabled)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)
  const connectionStatus = useConnectionStore(state => state.status)

  const port = config?.tgWsProxyPort ?? DEFAULT_TG_WS_PROXY_PORT
  const secret = normalizeTgWsProxySecret(config?.tgWsProxySecret)
  const tgLink = buildTgWsProxyLink(port, secret)
  const enabled = config?.tgWsProxyModuleEnabled ?? false

  const refreshStatus = async () => {
    const nextStatus = await tauri.getTgWsProxyStatus()
    setStatus(nextStatus)
    return nextStatus
  }

  const resolveStatus = async () => status ?? refreshStatus()

  const applyConfigState = (nextPort: number, nextSecret: string, nextEnabled = enabled) => {
    setTgWsProxyPort(nextPort)
    setTgWsProxySecret(normalizeTgWsProxySecret(nextSecret))
    setTgWsProxyModuleEnabled(nextEnabled)
  }

  const applySettings = async (nextPort: number, nextSecret: string) => {
    if (!config) {
      return false
    }

    if (!Number.isInteger(nextPort) || nextPort < 1 || nextPort > 65535) {
      toast.error('Порт должен быть в диапазоне 1-65535')
      return false
    }

    if (!isValidTgWsProxySecret(nextSecret)) {
      toast.error('Секрет должен состоять из 32 шестнадцатеричных символов')
      return false
    }

    const previousConfig = structuredClone(config)
    const normalizedSecret = normalizeTgWsProxySecret(nextSecret)

    applyConfigState(nextPort, normalizedSecret, enabled)

    let currentStatus: TgWsProxyStatus
    try {
      currentStatus = await resolveStatus()
      setIsBusy(true)
      await saveNow()
    }
    catch (error) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения параметров TG WS Proxy: ${error instanceof Error ? error.message : String(error)}`)
      await refreshStatus().catch(() => {})
      setIsBusy(false)
      return false
    }

    if (!enabled || connectionStatus !== 'connected') {
      toast.success('Параметры TG WS Proxy сохранены')
      setIsBusy(false)
      return true
    }

    try {
      if (currentStatus.running) {
        await tauri.stopTgWsProxy()
      }
      const nextStatus = await tauri.startTgWsProxy(nextPort, normalizedSecret)
      setStatus(nextStatus)
      addConfigLog(`TG WS Proxy перезапущен на порту ${nextPort}`)
      toast.success('Параметры TG WS Proxy применены')
      return true
    }
    catch (error) {
      toast.error(`Ошибка применения параметров TG WS Proxy: ${error instanceof Error ? error.message : String(error)}`)
      await refreshStatus().catch(() => {})
      return false
    }
    finally {
      setIsBusy(false)
    }
  }

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

    applyConfigState(port, secret, nextEnabled)

    let currentStatus: TgWsProxyStatus
    try {
      currentStatus = await resolveStatus()
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
          ? nextEnabled ? 'TG WS Proxy модуль включён и запущен' : 'TG WS Proxy модуль выключен и остановлен'
          : nextEnabled ? 'TG WS Proxy модуль включён и будет запущен при подключении' : 'TG WS Proxy модуль выключен',
      )
    }
    catch (error) {
      revertTo(previousConfig)
      await saveNow().catch(() => {})
      toast.error(`Ошибка переключения TG WS Proxy: ${error instanceof Error ? error.message : String(error)}`)
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
      const nextStatus = await tauri.getTgWsProxyStatus()
      if (isMounted) {
        setStatus(nextStatus)
      }
    }

    void init().catch((error) => {
      if (isMounted) {
        toast.error(`Ошибка инициализации TG WS Proxy: ${error instanceof Error ? error.message : String(error)}`)
      }
    })

    return () => {
      isMounted = false
    }
  })

  return {
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
  }
}
