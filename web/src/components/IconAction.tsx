import type { ComponentProps, ReactNode } from 'react'
import { Button } from '@/components/ui/button'
import { Kbd } from '@/components/ui/kbd'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'

type Props = Omit<ComponentProps<typeof Button>, 'children'> & {
  label: string
  shortcut?: string
  pressed?: boolean
  children: ReactNode
}

/** An icon-only button that still says what it does, with its keyboard shortcut. */
export function IconAction({ label, shortcut, pressed, className, children, ...props }: Props) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={label}
          aria-pressed={pressed}
          className={cn('text-muted-foreground hover:text-foreground', pressed && 'text-foreground [&_svg]:fill-current', className)}
          {...props}
        >
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent className="flex items-center gap-2">
        {label}
        {shortcut && <Kbd>{shortcut}</Kbd>}
      </TooltipContent>
    </Tooltip>
  )
}
