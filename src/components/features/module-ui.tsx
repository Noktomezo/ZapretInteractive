import type { ComponentType, ReactNode } from 'react'
import { CardAction, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Label } from '@/components/ui/label'

export const MODULE_PAGE_CARD_CLASS = 'gap-0! rounded-lg! border! border-border/60! bg-card! py-0! shadow-none! backdrop-blur-none!'

export function ModuleSectionHeader({
  icon: Icon,
  title,
  description,
  action,
}: {
  icon: ComponentType<{ className?: string }>
  title: ReactNode
  description: ReactNode
  action?: ReactNode
}) {
  return (
    <CardHeader className="flex! flex-row! items-center! gap-3! border-b border-border/60 p-4!">
      <div className="text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-muted/25">
        <Icon className="size-4" />
      </div>
      <div className="min-w-0 flex-1">
        <CardTitle className="font-sans text-sm leading-5 font-normal tracking-normal">{title}</CardTitle>
        <CardDescription className="mt-1 text-xs leading-4">{description}</CardDescription>
      </div>
      {action ? <CardAction className="self-center">{action}</CardAction> : null}
    </CardHeader>
  )
}

export function ModuleSettingLabel({
  htmlFor,
  icon: Icon,
  description,
  children,
}: {
  htmlFor: string
  icon: ComponentType<{ className?: string }>
  description?: ReactNode
  children: ReactNode
}) {
  return (
    <div className="flex items-center gap-3">
      <div className="text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-muted/25">
        <Icon className="size-4" />
      </div>
      <div className="min-w-0 flex-1">
        <Label htmlFor={htmlFor} className="text-sm leading-5 font-normal">
          {children}
        </Label>
        {description
          ? <p className="mt-1 text-xs leading-4 text-muted-foreground">{description}</p>
          : null}
      </div>
    </div>
  )
}
