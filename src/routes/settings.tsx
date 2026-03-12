import { createRoute } from '@tanstack/react-router'
import { lazyRouteComponent } from '@/lib/lazy-route'
import { Route as rootRoute } from './__root'

const SettingsRoute = lazyRouteComponent(
  () => import('../components/features/SettingsPage'),
  'SettingsPage',
)

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/settings',
  component: SettingsRoute,
})
