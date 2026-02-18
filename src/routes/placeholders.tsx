import { createRoute } from '@tanstack/react-router'
import { PlaceholdersPage } from '../components/features/PlaceholdersPage'
import { Route as rootRoute } from './__root'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/placeholders',
  component: PlaceholdersPage,
})
