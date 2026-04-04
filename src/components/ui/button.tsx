import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'
import { Slot } from 'radix-ui'
import * as React from 'react'

import { cn } from '@/lib/utils'

const buttonVariants = cva(
  'cursor-pointer inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-[13px] font-medium tracking-[-0.015em] transition-all duration-200 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*=\'size-\'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive',
  {
    variants: {
      variant: {
        default: 'bg-primary text-primary-foreground shadow-[0_10px_24px_color-mix(in_oklab,var(--primary)_28%,transparent)] hover:bg-primary/92 hover:shadow-[0_14px_28px_color-mix(in_oklab,var(--primary)_34%,transparent)]',
        destructive:
          'border border-destructive/35 bg-destructive/78 text-destructive-foreground shadow-[0_10px_24px_color-mix(in_oklab,var(--destructive)_24%,transparent)] hover:border-destructive/45 hover:bg-destructive/88 hover:shadow-[0_14px_28px_color-mix(in_oklab,var(--destructive)_32%,transparent)] focus-visible:ring-destructive/20 dark:border-destructive/30 dark:bg-destructive/62 dark:hover:bg-destructive/72 dark:focus-visible:ring-destructive/40',
        outline:
          'border border-border/80 bg-background/92 text-foreground shadow-xs hover:border-border hover:bg-background dark:bg-input/30 dark:hover:bg-input/50',
        secondary:
          'border border-border/70 bg-background/88 text-secondary-foreground shadow-xs hover:border-border hover:bg-background dark:bg-input/24 dark:hover:bg-input/42',
        ghost:
          'text-foreground/85 hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/70',
        link: 'text-primary underline-offset-4 hover:underline',
      },
      size: {
        'default': 'h-9 px-3.5 py-2 has-[>svg]:px-3',
        'xs': 'h-7 gap-1 rounded-md px-2.5 text-xs has-[>svg]:px-2 [&_svg:not([class*=\'size-\'])]:size-3',
        'sm': 'h-8 gap-1.5 px-3 has-[>svg]:px-2.5',
        'lg': 'h-10 px-5 has-[>svg]:px-3.5',
        'icon': 'size-9',
        'icon-xs': 'size-7 rounded-md [&_svg:not([class*=\'size-\'])]:size-3',
        'icon-sm': 'size-8',
        'icon-lg': 'size-10',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  },
)

function Button({
  className,
  variant = 'default',
  size = 'default',
  asChild = false,
  ...props
}: React.ComponentProps<'button'>
  & VariantProps<typeof buttonVariants> & {
    asChild?: boolean
  }) {
  const Comp = asChild ? Slot.Root : 'button'

  return (
    <Comp
      data-slot="button"
      data-variant={variant}
      data-size={size}
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  )
}

export { Button, buttonVariants }
