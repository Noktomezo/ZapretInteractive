import { createRoute } from '@tanstack/react-router'
import { Route as rootRoute } from './__root'
import { PlaceholdersPage } from '../components/features/PlaceholdersPage'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/placeholders',
  component: PlaceholdersPage,
})