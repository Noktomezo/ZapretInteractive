'use client'

import { Switch as SwitchPrimitive } from 'radix-ui'
import * as React from 'react'

import { cn } from '@/lib/utils'

function Switch({
  className,
  size = 'default',
  checked,
  defaultChecked,
  disabled,
  onCheckedChange,
  ...props
}: React.ComponentProps<typeof SwitchPrimitive.Root> & {
  size?: 'sm' | 'default'
}) {
  const isControlled = checked !== undefined
  const [internalChecked, setInternalChecked] = React.useState(Boolean(defaultChecked))
  const isChecked = isControlled ? checked : internalChecked

  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      data-size={size}
      checked={isControlled ? checked : undefined}
      defaultChecked={defaultChecked}
      disabled={disabled}
      onCheckedChange={(nextChecked) => {
        if (!isControlled) {
          setInternalChecked(nextChecked)
        }
        onCheckedChange?.(nextChecked)
      }}
      className={cn(
        'peer group/switch inline-flex w-fit cursor-pointer shrink-0 items-center gap-2 rounded-md border border-border/60 bg-background/92 px-2 text-foreground shadow-xs transition-[background-color,border-color,box-shadow,color] outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/35 disabled:cursor-not-allowed disabled:opacity-50 dark:bg-input/30 data-[size=default]:h-9 data-[size=sm]:h-8',
        className,
      )}
      {...props}
    >
      <span className="text-[11px] leading-none font-medium whitespace-nowrap">
        {isChecked ? 'Вкл' : 'Выкл'}
      </span>
      <span
        aria-hidden="true"
        className={cn(
          'flex shrink-0 items-center rounded-sm bg-muted p-0.5 transition-colors',
          'group-data-[state=checked]/switch:bg-success group-data-[state=unchecked]/switch:bg-destructive',
          'group-data-[size=default]/switch:h-5 group-data-[size=default]/switch:w-9',
          'group-data-[size=sm]/switch:h-4 group-data-[size=sm]/switch:w-7',
        )}
      >
        <SwitchPrimitive.Thumb
          data-slot="switch-thumb"
          className={cn(
            'pointer-events-none block rounded-sm bg-background ring-0 transition-transform group-data-[size=default]/switch:size-4 group-data-[size=default]/switch:group-data-[state=checked]/switch:translate-x-4 group-data-[size=sm]/switch:size-2.5 group-data-[size=sm]/switch:group-data-[state=checked]/switch:translate-x-3.5 group-data-[state=unchecked]/switch:translate-x-0',
          )}
        />
      </span>
    </SwitchPrimitive.Root>
  )
}

export { Switch }
