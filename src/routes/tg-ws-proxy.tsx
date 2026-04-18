import { createRoute } from '@tanstack/react-router'
import { TgWsProxyPage } from '@/components/features/TgWsProxyPage'
import { Route as rootRoute } from './__root'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/modules/tg-ws-proxy',
  component: TgWsProxyPage,
})
