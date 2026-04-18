import { createRoute } from '@tanstack/react-router'
import { AboutPage } from '@/components/features/AboutPage'
import { Route as rootRoute } from './__root'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/about',
  component: AboutPage,
})
