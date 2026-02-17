import { createRoute } from '@tanstack/react-router'
import { Route as rootRoute } from './__root'
import { MainPage } from '../components/features/MainPage'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: MainPage,
})