import { createRoute } from '@tanstack/react-router'
import { DnsPage } from '@/components/features/DnsPage'
import { Route as rootRoute } from './__root'

export const Route = createRoute({
  getParentRoute: () => rootRoute,
  path: '/modules/dns',
  component: DnsPage,
})
