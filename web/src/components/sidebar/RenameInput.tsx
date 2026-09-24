import { useRef, useState } from 'react'
import { SidebarInput } from '@/components/ui/sidebar'

type Props = {
  initial: string
  label: string
  /** Receives the trimmed name, or `undefined` when cancelled. */
  onDone: (value: string | undefined) => void
}

export function RenameInput({ initial, label, onDone }: Props) {
  const [value, setValue] = useState(initial)
  // Enter finishes and unmounts the input, which also fires blur; only the first one counts.
  const finished = useRef(false)
  const finish = (result: string | undefined) => {
    if (finished.current) return
    finished.current = true
    onDone(result)
  }
  return (
    <SidebarInput
      aria-label={label}
      placeholder={label}
      autoFocus
      value={value}
      onChange={(event) => setValue(event.target.value)}
      onFocus={(event) => event.target.select()}
      onKeyDown={(event) => {
        if (event.key === 'Enter') finish(value.trim())
        if (event.key === 'Escape') finish(undefined)
      }}
      onBlur={() => finish(value.trim())}
      className="h-8 border-signal bg-background"
    />
  )
}
