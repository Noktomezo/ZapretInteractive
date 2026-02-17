import { createRoute } from '@tanstack/react-router'
import { Route as rootRoute } from './__root'
import { FiltersPage } from '../components/features/FiltersPage'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/filters',
  component: FiltersPage,
})
