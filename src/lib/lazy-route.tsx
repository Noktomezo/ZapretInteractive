import type { ComponentType } from 'react'
import { lazy, Suspense } from 'react'

export function lazyRouteComponent<T extends Record<string, ComponentType<any>>>(
  importer: () => Promise<T>,
  exportName: keyof T,
) {
  const LazyComponent = lazy(async () => {
    const mod = await importer()
    return { default: mod[exportName] as ComponentType }
  })

  return function LazyRouteComponent() {
    return (
      <Suspense
        fallback={(
          <div className="flex h-full items-center justify-center">
            <div className="h-6 w-6 animate-spin rounded-full border-2 border-current border-t-transparent" />
          </div>
        )}
      >
        <LazyComponent />
      </Suspense>
    )
  }
}
