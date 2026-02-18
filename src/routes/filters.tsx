import { createRoute } from '@tanstack/react-router'
import { FiltersPage } from '../components/features/FiltersPage'
import { Route as rootRoute } from './__root'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/filters',
  component: FiltersPage,
})
