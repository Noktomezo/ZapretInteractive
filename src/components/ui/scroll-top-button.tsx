import type { RefObject } from 'react'
import type { LenisLike } from '@/lib/editor-scroll'
import { ArrowUp } from 'lucide-react'
import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'

const VIEWPORT_SELECTOR = '[data-slot="lenis-scroll-area-viewport"], [data-slot="scroll-area-viewport"]'
const SHOW_AFTER_SCROLL_PX = 320

interface ScrollTopButtonProps {
  scrollAreaRef: RefObject<HTMLElement | null>
  /** Values that re-arm the scroll listener when changed (e.g. route id, loading flag). */
  resetKeys?: readonly (string | number | boolean | null | undefined)[]
}

function getViewport(scrollAreaRef: RefObject<HTMLElement | null>) {
  return scrollAreaRef.current?.querySelector<HTMLDivElement>(VIEWPORT_SELECTOR) ?? null
}

export function ScrollTopButton({ scrollAreaRef, resetKeys = [] }: ScrollTopButtonProps) {
  const [visible, setVisible] = useState(false)
  const resetKey = resetKeys.map(value => String(value)).join('|')

  useEffect(() => {
    const viewport = getViewport(scrollAreaRef)
    if (!viewport) {
      setVisible(false)
      return
    }

    const handleScroll = () => {
      setVisible(viewport.scrollTop > SHOW_AFTER_SCROLL_PX)
    }

    handleScroll()
    viewport.addEventListener('scroll', handleScroll, { passive: true })
    return () => {
      viewport.removeEventListener('scroll', handleScroll)
    }
  }, [scrollAreaRef, resetKey])

  if (!visible) {
    return null
  }

  const scrollToTop = () => {
    const viewport = getViewport(scrollAreaRef)
    if (!viewport) {
      return
    }

    const lenis = (viewport as HTMLDivElement & { __lenis?: LenisLike }).__lenis
    if (lenis) {
      lenis.scrollTo(0, {
        duration: 0.45,
        easing: value => 1 - (1 - value) ** 3,
      })
      return
    }

    viewport.scrollTo({ top: 0, behavior: 'smooth' })
  }

  return (
    <Button
      type="button"
      size="default"
      variant="secondary"
      className="absolute bottom-5 left-1/2 z-20 -translate-x-1/2 border border-border bg-background/60 text-foreground shadow-lg backdrop-blur-md hover:bg-background/72 hover:backdrop-blur-xl dark:bg-card/60 dark:hover:bg-card/72"
      aria-label="Вернуться наверх"
      onClick={scrollToTop}
    >
      <ArrowUp className="size-4" />
      Наверх
    </Button>
  )
}
