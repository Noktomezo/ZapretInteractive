import { createRoute } from '@tanstack/react-router'
import { MainPage } from '../components/features/MainPage'
import { Route as rootRoute } from './__root'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: MainPage,
})
