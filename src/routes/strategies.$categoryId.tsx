import { createRoute } from '@tanstack/react-router'
import { CategoryPage } from '../components/features/CategoryPage'
import { Route as rootRoute } from './__root'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/strategies/$categoryId',
  component: CategoryPage,
})
