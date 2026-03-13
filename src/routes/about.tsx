import { createRoute } from '@tanstack/react-router'
import { lazyRouteComponent } from '@/lib/lazy-route'
import { Route as rootRoute } from './__root'

const AboutRoute = lazyRouteComponent(
  () => import('../components/features/AboutPage'),
  'AboutPage',
)

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/about',
  component: AboutRoute,
})
