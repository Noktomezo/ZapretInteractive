import { createRoute } from '@tanstack/react-router'
import { lazyRouteComponent } from '@/lib/lazy-route'
import { Route as rootRoute } from './__root'

const ModulesRoute = lazyRouteComponent(
  () => import('../components/features/ModulesPage'),
  'ModulesPage',
)

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/modules',
  component: ModulesRoute,
})
