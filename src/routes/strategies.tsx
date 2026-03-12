import { createRoute } from '@tanstack/react-router'
import { lazyRouteComponent } from '@/lib/lazy-route'
import { Route as rootRoute } from './__root'

const CategoriesListRoute = lazyRouteComponent(
  () => import('../components/features/CategoriesListPage'),
  'CategoriesListPage',
)

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/strategies',
  component: CategoriesListRoute,
})
