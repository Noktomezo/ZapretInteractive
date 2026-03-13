import { createRouter, RouterProvider } from '@tanstack/react-router'
import ReactDOM from 'react-dom/client'
import { Route as rootRoute } from './routes/__root'
import { Route as aboutRoute } from './routes/about'
import { Route as filtersRoute } from './routes/filters'
import { Route as indexRoute } from './routes/index'
import { Route as logsRoute } from './routes/logs'
import { Route as placeholdersRoute } from './routes/placeholders'
import { Route as settingsRoute } from './routes/settings'
import { Route as strategiesRoute } from './routes/strategies'
import { Route as strategiesCategoryIdRoute } from './routes/strategies.$categoryId'
import './index.css'

const routeTree = rootRoute.addChildren([
  indexRoute,
  aboutRoute,
  strategiesRoute,
  strategiesCategoryIdRoute,
  filtersRoute,
  placeholdersRoute,
  logsRoute,
  settingsRoute,
])

const router = createRouter({
  routeTree,
  defaultPreload: 'intent',
})

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <RouterProvider router={router} />,
)
