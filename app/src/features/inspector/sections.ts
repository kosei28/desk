import { createSignal } from "solid-js"

export type SectionMemory = ReturnType<typeof createSectionMemory>

/**
 * 右の列の見出しの開け閉め（[情報設計の決まり] の「右の列」）。見出しの言葉ごとに覚え、ほかのファイルに移っても保つ。
 * どれも開いて始める。アプリを閉じたら残さない
 */
export function createSectionMemory() {
  const [closed, setClosed] = createSignal<ReadonlySet<string>>(new Set())
  return {
    isOpen: (title: string) => !closed().has(title),
    setOpen(title: string, open: boolean) {
      setClosed((current) => {
        if (current.has(title) !== open) return current
        const next = new Set(current)
        if (open) next.delete(title)
        else next.add(title)
        return next
      })
    },
  }
}
