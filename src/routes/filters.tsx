import { createRoute } from '@tanstack/react-router'
import { lazyRouteComponent } from '@/lib/lazy-route'
import { Route as rootRoute } from './__root'

const FiltersRoute = lazyRouteComponent(
  () => import('../components/features/FiltersPage'),
  'FiltersPage',
)

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/filters',
  component: FiltersRoute,
})
