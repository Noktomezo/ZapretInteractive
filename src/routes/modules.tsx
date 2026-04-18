import { createRoute } from '@tanstack/react-router'
import { ModulesPage } from '@/components/features/ModulesPage'
import { Route as rootRoute } from './__root'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/modules',
  component: ModulesPage,
})
