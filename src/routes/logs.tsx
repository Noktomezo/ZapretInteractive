import { createRoute } from '@tanstack/react-router'
import { LogsPage } from '../components/features/LogsPage'
import { Route as rootRoute } from './__root'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/logs',
  component: LogsPage,
})
