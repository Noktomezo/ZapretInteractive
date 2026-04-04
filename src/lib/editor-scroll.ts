import type * as React from 'react'

interface LenisLike {
  scrollTo: (target: number, options?: { duration?: number, easing?: (value: number) => number }) => void
}

export function autosizeTextarea(textarea: HTMLTextAreaElement | null) {
  if (!textarea) {
    return
  }

  textarea.style.height = '0px'
  textarea.style.height = `${textarea.scrollHeight}px`
}

export function forwardTextareaWheelToScrollArea(event: React.WheelEvent<HTMLTextAreaElement>) {
  const scrollAreaRoot = event.currentTarget.closest('[data-slot="lenis-scroll-area"], [data-slot="scroll-area"]')
  const viewport = scrollAreaRoot?.querySelector(
    '[data-slot="lenis-scroll-area-viewport"], [data-slot="scroll-area-viewport"]',
  ) as (HTMLDivElement & { __lenis?: LenisLike }) | null

  if (!viewport || viewport.scrollHeight <= viewport.clientHeight) {
    return
  }

  event.preventDefault()

  const nextTop = Math.max(0, Math.min(
    viewport.scrollTop + event.deltaY,
    viewport.scrollHeight - viewport.clientHeight,
  ))

  if (viewport.__lenis) {
    viewport.__lenis.scrollTo(nextTop, {
      duration: 0.18,
      easing: value => 1 - (1 - value) ** 2,
    })
    return
  }

  viewport.scrollTo({ top: nextTop, behavior: 'smooth' })
}
