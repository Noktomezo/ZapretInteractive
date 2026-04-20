import { createRouter, RouterProvider } from '@tanstack/react-router'
import ReactDOM from 'react-dom/client'
import { getDiscordPresenceDetails, getDiscordPresenceState } from './lib/discord-presence'
import * as tauri from './lib/tauri'
import { Route as rootRoute } from './routes/__root'
import { Route as aboutRoute } from './routes/about'
import { Route as dnsRoute } from './routes/dns'
import { Route as filtersRoute } from './routes/filters'
import { Route as indexRoute } from './routes/index'
import { Route as logsRoute } from './routes/logs'
import { Route as modulesRoute } from './routes/modules'
import { Route as placeholdersRoute } from './routes/placeholders'
import { Route as settingsRoute } from './routes/settings'
import { Route as strategiesRoute } from './routes/strategies'
import { Route as strategiesCategoryIdRoute } from './routes/strategies.$categoryId'
import { Route as tgWsProxyRoute } from './routes/tg-ws-proxy'
import { useConfigStore } from './stores/config.store'
import { useConnectionStore } from './stores/connection.store'
import './index.css'

const routeTree = rootRoute.addChildren([
  indexRoute,
  aboutRoute,
  modulesRoute,
  dnsRoute,
  tgWsProxyRoute,
  strategiesRoute,
  strategiesCategoryIdRoute,
  filtersRoute,
  placeholdersRoute,
  logsRoute,
  settingsRoute,
])

const router = createRouter({
  routeTree,
  defaultPreload: 'intent',
})

type DiscordPresenceSyncGlobal = typeof globalThis & {
  __zapretDiscordPresenceSyncCleanup__?: () => void
}

let lastDiscordPresenceKey: string | null = null
let discordPresenceSyncPromise: Promise<void> = Promise.resolve()
let discordPresenceRetryTimeoutId: number | null = null

function clearDiscordPresenceRetryTimeout() {
  if (discordPresenceRetryTimeoutId !== null) {
    window.clearTimeout(discordPresenceRetryTimeoutId)
    discordPresenceRetryTimeoutId = null
  }
}

function scheduleDiscordPresenceRetry() {
  clearDiscordPresenceRetryTimeout()
  discordPresenceRetryTimeoutId = window.setTimeout(() => {
    discordPresenceRetryTimeoutId = null
    syncDiscordPresenceState(true)
  }, 5000)
}

function syncDiscordPresenceState(force = false) {
  const config = useConfigStore.getState().config
  if (!config) {
    return
  }

  const enabled = config.discordPresenceEnabled ?? false
  const activityType = config.discordPresenceActivityType ?? 'playing'
  const details = getDiscordPresenceDetails(router.state.location.pathname, config)
  const state = getDiscordPresenceState(useConnectionStore.getState().status)
  const nextKey = JSON.stringify([enabled, activityType, details, state])

  if (!force && nextKey === lastDiscordPresenceKey) {
    return
  }

  discordPresenceSyncPromise = discordPresenceSyncPromise
    .catch(() => {})
    .then(async () => {
      const synced = await tauri.syncDiscordPresence(enabled, details, state, activityType)
      if (!enabled || synced) {
        lastDiscordPresenceKey = nextKey
        clearDiscordPresenceRetryTimeout()
      }
      else {
        scheduleDiscordPresenceRetry()
      }
    })
    .catch((error) => {
      console.error('Failed to sync Discord presence:', error)
      scheduleDiscordPresenceRetry()
    })
}

const discordPresenceSyncGlobal = globalThis as DiscordPresenceSyncGlobal
discordPresenceSyncGlobal.__zapretDiscordPresenceSyncCleanup__?.()

const cleanupDiscordPresenceRouteSubscription = router.subscribe('onResolved', () => {
  syncDiscordPresenceState()
})

const cleanupDiscordPresenceConfigSubscription = useConfigStore.subscribe((state, previousState) => {
  if (state.config !== previousState.config) {
    syncDiscordPresenceState()
  }
})

const cleanupDiscordPresenceConnectionSubscription = useConnectionStore.subscribe((state, previousState) => {
  if (state.status !== previousState.status) {
    syncDiscordPresenceState()
  }
})

const discordPresenceRefreshIntervalId = window.setInterval(() => {
  syncDiscordPresenceState(true)
}, 30000)

discordPresenceSyncGlobal.__zapretDiscordPresenceSyncCleanup__ = () => {
  clearDiscordPresenceRetryTimeout()
  window.clearInterval(discordPresenceRefreshIntervalId)
  cleanupDiscordPresenceRouteSubscription()
  cleanupDiscordPresenceConfigSubscription()
  cleanupDiscordPresenceConnectionSubscription()
}

syncDiscordPresenceState()

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <RouterProvider router={router} />,
)
