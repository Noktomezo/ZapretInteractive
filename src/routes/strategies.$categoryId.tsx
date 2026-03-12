import { createRoute } from '@tanstack/react-router'
import { lazyRouteComponent } from '@/lib/lazy-route'
import { Route as rootRoute } from './__root'

const CategoryRoute = lazyRouteComponent(
  () => import('../components/features/CategoryPage'),
  'CategoryPage',
)

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/strategies/$categoryId',
  component: CategoryRoute,
})
