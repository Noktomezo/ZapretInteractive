import { createRoute } from '@tanstack/react-router'
import { SettingsPage } from '../components/features/SettingsPage'
import { Route as rootRoute } from './__root'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/settings',
  component: SettingsPage,
})
