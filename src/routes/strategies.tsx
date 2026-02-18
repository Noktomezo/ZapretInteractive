import { createRoute } from '@tanstack/react-router'
import { CategoriesListPage } from '../components/features/CategoriesListPage'
import { Route as rootRoute } from './__root'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/strategies',
  component: CategoriesListPage,
})
