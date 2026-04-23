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
        'peer group/switch inline-flex cursor-pointer shrink-0 items-center justify-between gap-2 rounded-md border bg-background/92 px-2 text-foreground shadow-xs transition-[background-color,border-color,box-shadow,color] outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 dark:bg-input/30 data-[state=checked]:border-success/50 data-[state=unchecked]:border-destructive/45 dark:data-[state=unchecked]:border-destructive/35 data-[size=default]:h-9 data-[size=default]:min-w-[4.75rem] data-[size=sm]:h-8 data-[size=sm]:min-w-[4.25rem]',
        className,
      )}
      {...props}
    >
      <span className="min-w-[2rem] text-[11px] font-medium leading-none">
        {isChecked ? 'Вкл' : 'Выкл'}
      </span>
      <span
        aria-hidden="true"
        className={cn(
          'flex shrink-0 items-center rounded-sm border p-0.5 transition-[background-color,border-color]',
          'group-data-[state=checked]/switch:border-success/60 group-data-[state=checked]/switch:bg-success',
          'group-data-[state=unchecked]/switch:border-destructive/35 group-data-[state=unchecked]/switch:bg-destructive/72 dark:group-data-[state=unchecked]/switch:border-destructive/30 dark:group-data-[state=unchecked]/switch:bg-destructive/58',
          'group-data-[size=default]/switch:h-[1.15rem] group-data-[size=default]/switch:w-8',
          'group-data-[size=sm]/switch:h-4 group-data-[size=sm]/switch:w-6',
        )}
      >
        <SwitchPrimitive.Thumb
          data-slot="switch-thumb"
          className={cn(
            'pointer-events-none block rounded-sm bg-background ring-0 transition-transform group-data-[size=default]/switch:size-3.5 group-data-[size=sm]/switch:size-2.5 group-data-[state=checked]/switch:translate-x-[calc(100%-2px)] group-data-[state=unchecked]/switch:translate-x-0',
          )}
        />
      </span>
    </SwitchPrimitive.Root>
  )
}

export { Switch }
