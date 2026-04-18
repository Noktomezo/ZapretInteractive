import { createRoute } from '@tanstack/react-router'
import { lazyRouteComponent } from '@/lib/lazy-route'
import { Route as rootRoute } from './__root'

const DnsRoute = lazyRouteComponent(
  () => import('../components/features/DnsPage'),
  'DnsPage',
)

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/modules/dns',
  component: DnsRoute,
})
