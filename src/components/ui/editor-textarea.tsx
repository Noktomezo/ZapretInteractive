import * as React from 'react'

import { forwardTextareaWheelToScrollArea } from '@/lib/editor-scroll'
import { cn } from '@/lib/utils'
import { LenisScrollArea } from './lenis-scroll-area'
import { Textarea } from './textarea'

interface EditorTextareaProps extends React.ComponentProps<'textarea'> {
  scrollAreaClassName?: string
  scrollAreaContentClassName?: string
  textareaRef?: React.Ref<HTMLTextAreaElement>
}

function EditorTextarea({
  className,
  onWheel,
  scrollAreaClassName,
  scrollAreaContentClassName,
  textareaRef: forwardedRef,
  ...props
}: EditorTextareaProps) {
  const textareaRef = React.useRef<HTMLTextAreaElement | null>(null)
  const setTextareaRef = React.useCallback((node: HTMLTextAreaElement | null) => {
    textareaRef.current = node

    if (typeof forwardedRef === 'function') {
      forwardedRef(node)
      return
    }

    if (forwardedRef) {
      forwardedRef.current = node
    }
  }, [forwardedRef])

  return (
    <LenisScrollArea
      className={cn(
        'max-h-[calc(100vh-22rem)] rounded-md border border-border/80 bg-background/92 shadow-xs transition-[border-color,box-shadow,background-color] hover:border-border focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50 dark:bg-input/30',
        scrollAreaClassName,
      )}
      contentClassName={cn('cursor-text', scrollAreaContentClassName)}
      onClick={() => textareaRef.current?.focus()}
    >
      <Textarea
        data-lenis-prevent
        ref={setTextareaRef}
        onWheel={(event) => {
          onWheel?.(event)
          if (!event.defaultPrevented) {
            forwardTextareaWheelToScrollArea(event)
          }
        }}
        className={cn(
          'resize-none overflow-hidden rounded-none border-0 bg-transparent px-3 py-3 font-mono text-sm shadow-none hover:border-transparent focus-visible:border-transparent focus-visible:ring-0',
          className,
        )}
        {...props}
      />
    </LenisScrollArea>
  )
}

export { EditorTextarea }
