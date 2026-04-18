import { createRoute } from '@tanstack/react-router'
import { lazyRouteComponent } from '@/lib/lazy-route'
import { Route as rootRoute } from './__root'

const TgWsProxyRoute = lazyRouteComponent(
  () => import('../components/features/TgWsProxyPage'),
  'TgWsProxyPage',
)

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/modules/tg-ws-proxy',
  component: TgWsProxyRoute,
})
