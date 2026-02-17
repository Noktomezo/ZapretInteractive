import { createRoute } from '@tanstack/react-router'
import { Route as rootRoute } from './__root'
import { SettingsPage } from '../components/features/SettingsPage'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/settings',
  component: SettingsPage,
})