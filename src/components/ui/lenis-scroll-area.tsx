import Lenis from 'lenis'
import { ScrollArea as ScrollAreaPrimitive } from 'radix-ui'
import * as React from 'react'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { cn } from '@/lib/utils'
import { ScrollBar } from './scroll-area'

interface LenisScrollAreaProps extends React.ComponentProps<typeof ScrollAreaPrimitive.Root> {
  contentClassName?: string
}

const LenisScrollArea = React.forwardRef<
  React.ElementRef<typeof ScrollAreaPrimitive.Root> | null,
  LenisScrollAreaProps
>(({ className, contentClassName, children, ...props }, forwardedRef) => {
  const rootRef = React.useRef<React.ElementRef<typeof ScrollAreaPrimitive.Root> | null>(null)
  const viewportRef = React.useRef<React.ElementRef<typeof ScrollAreaPrimitive.Viewport> | null>(null)
  const contentRef = React.useRef<HTMLDivElement | null>(null)

  React.useImperativeHandle<
    React.ElementRef<typeof ScrollAreaPrimitive.Root> | null,
    React.ElementRef<typeof ScrollAreaPrimitive.Root> | null
  >(forwardedRef, () => rootRef.current, [])

  useMountEffect(() => {
    const wrapper = viewportRef.current
    const content = contentRef.current
    if (!wrapper || !content) {
      return
    }

    const lenis = new Lenis({
      wrapper,
      content,
      eventsTarget: wrapper,
      autoRaf: true,
      smoothWheel: true,
      syncTouch: false,
      overscroll: true,
      allowNestedScroll: true,
      lerp: 0.12,
      prevent: node => node instanceof HTMLElement && node.closest('[data-lenis-prevent]') !== null,
    })
    ;(wrapper as HTMLElement & { __lenis?: Lenis }).__lenis = lenis

    return () => {
      delete (wrapper as HTMLElement & { __lenis?: Lenis }).__lenis
      lenis.destroy()
    }
  })

  return (
    <ScrollAreaPrimitive.Root
      ref={rootRef}
      data-slot="lenis-scroll-area"
      className={cn('relative min-h-0 min-w-0 overflow-hidden', className)}
      {...props}
    >
      <ScrollAreaPrimitive.Viewport
        ref={viewportRef}
        data-slot="lenis-scroll-area-viewport"
        className="focus-visible:ring-ring/50 size-full rounded-[inherit] transition-[color,box-shadow] outline-none focus-visible:ring-[3px] focus-visible:outline-1"
      >
        <div ref={contentRef} className={cn('min-h-full min-w-0 w-full max-w-full', contentClassName)}>
          {children}
        </div>
      </ScrollAreaPrimitive.Viewport>
      <ScrollBar />
      <ScrollAreaPrimitive.Corner />
    </ScrollAreaPrimitive.Root>
  )
})

LenisScrollArea.displayName = 'LenisScrollArea'

export { LenisScrollArea }
