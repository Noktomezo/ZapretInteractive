import type { MarkdownToJSX } from 'markdown-to-jsx'
import type { ComponentPropsWithoutRef } from 'react'
import { openUrl } from '@tauri-apps/plugin-opener'
import Markdown from 'markdown-to-jsx'
import { toast } from 'sonner'
import { cn } from '@/lib/utils'

async function handleExternalMarkdownLink(url: string) {
  try {
    await openUrl(url)
  }
  catch (e) {
    toast.error(`Не удалось открыть ссылку: ${e instanceof Error ? e.message : String(e)}`)
  }
}

function MarkdownLink({ href, children, ...props }: ComponentPropsWithoutRef<'a'>) {
  if (!href) {
    return <span>{children}</span>
  }

  return (
    <a
      {...props}
      href={href}
      className={cn('text-foreground underline underline-offset-2 transition-colors hover:text-primary', props.className)}
      onClick={(e) => {
        e.preventDefault()
        void handleExternalMarkdownLink(href)
      }}
    >
      {children}
    </a>
  )
}

const markdownOptions: MarkdownToJSX.Options = {
  disableParsingRawHTML: true,
  forceBlock: true,
  overrides: {
    a: { component: MarkdownLink },
    p: { props: { className: 'mb-3 leading-6 last:mb-0' } },
    ul: { props: { className: 'mb-3 list-disc space-y-1 pl-5 last:mb-0' } },
    ol: { props: { className: 'mb-3 list-decimal space-y-1 pl-5 last:mb-0' } },
    li: { props: { className: 'leading-6' } },
    blockquote: { props: { className: 'mb-3 border-l-2 border-border/80 pl-3 italic last:mb-0' } },
    code: { props: { className: 'rounded-sm bg-background/80 px-1.5 py-0.5 font-mono text-[11px] text-foreground' } },
    pre: { props: { className: 'mb-3 overflow-x-auto rounded-md border border-border/60 bg-background/70 p-3 font-mono text-[11px] text-foreground last:mb-0' } },
    h1: { props: { className: 'mb-3 text-sm font-semibold text-foreground last:mb-0' } },
    h2: { props: { className: 'mb-3 text-sm font-semibold text-foreground last:mb-0' } },
    h3: { props: { className: 'mb-2 text-xs font-semibold text-foreground last:mb-0' } },
    hr: { props: { className: 'my-3 border-border/60 last:mb-0' } },
    table: { props: { className: 'mb-3 w-full border-collapse text-left text-xs last:mb-0' } },
    thead: { props: { className: 'border-b border-border/60' } },
    th: { props: { className: 'px-2 py-1 font-medium text-foreground' } },
    td: { props: { className: 'border-t border-border/40 px-2 py-1 align-top' } },
  },
}

interface MarkdownContentProps {
  children: string
  className?: string
}

export function MarkdownContent({ children, className }: MarkdownContentProps) {
  return (
    <div className={cn('text-xs text-muted-foreground', className)}>
      <Markdown options={markdownOptions}>
        {children}
      </Markdown>
    </div>
  )
}
