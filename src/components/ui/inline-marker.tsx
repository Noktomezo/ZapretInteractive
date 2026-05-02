import type { LucideIcon } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Tooltip, TooltipContent, TooltipTrigger } from './tooltip'

export interface InlineMarkerProps {
  icon: LucideIcon
  label: string
  className?: string
  onClick?: () => void
}

export function InlineMarker({ icon: Icon, label, className, onClick }: InlineMarkerProps) {
  const content = onClick
    ? (
        <button
          type="button"
          onClick={(event) => {
            event.preventDefault()
            event.stopPropagation()
            onClick()
          }}
          className={cn('inline-flex h-4 w-4 cursor-pointer items-center justify-center transition-colors hover:text-foreground', className)}
          aria-label={label}
        >
          <Icon className="h-3.5 w-3.5" />
        </button>
      )
    : (
        <span
          className={cn('inline-flex h-4 w-4 cursor-help items-center justify-center', className)}
          role="img"
          aria-label={label}
          tabIndex={0}
        >
          <Icon className="h-3.5 w-3.5" />
        </span>
      )

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        {content}
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  )
}
