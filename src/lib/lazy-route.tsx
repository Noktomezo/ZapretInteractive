import { Loader2 } from 'lucide-react'
import type { ComponentType } from 'react'
import { Suspense, lazy } from 'react'

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
            <Loader2 className="h-6 w-6 animate-spin" />
          </div>
        )}
      >
        <LazyComponent />
      </Suspense>
    )
  }
}
