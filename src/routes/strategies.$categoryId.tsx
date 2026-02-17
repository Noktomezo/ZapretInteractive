import { createRoute } from '@tanstack/react-router'
import { Route as rootRoute } from './__root'
import { CategoryPage } from '../components/features/CategoryPage'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/strategies/$categoryId',
  component: CategoryPage,
})