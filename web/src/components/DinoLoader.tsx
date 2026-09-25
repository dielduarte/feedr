/** The feedrsauros dinosaur reading while something loads. */
export function DinoLoader({ label }: { label: string }) {
  return (
    <div role="status" aria-live="polite" className="flex flex-col items-center gap-3 py-20 text-center">
      <img src="/dino-loading.svg" alt="" width={96} height={96} className="size-24" />
      <p className="text-[15px] text-muted-foreground">{label}</p>
    </div>
  )
}
