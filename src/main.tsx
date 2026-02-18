import { createRouter, RouterProvider } from '@tanstack/react-router'
import React from 'react'
import ReactDOM from 'react-dom/client'
import { Route as rootRoute } from './routes/__root'
import { Route as filtersRoute } from './routes/filters'
import { Route as indexRoute } from './routes/index'
import { Route as placeholdersRoute } from './routes/placeholders'
import { Route as settingsRoute } from './routes/settings'
import { Route as strategiesRoute } from './routes/strategies'
import { Route as strategiesCategoryIdRoute } from './routes/strategies.$categoryId'
import './index.css'

const routeTree = rootRoute.addChildren([
  indexRoute,
  strategiesRoute,
  strategiesCategoryIdRoute,
  filtersRoute,
  placeholdersRoute,
  settingsRoute,
])

const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>,
)
