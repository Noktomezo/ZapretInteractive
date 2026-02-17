import { createRoute } from '@tanstack/react-router'
import { Route as rootRoute } from './__root'
import { CategoriesListPage } from '../components/features/CategoriesListPage'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/strategies',
  component: CategoriesListPage,
})