import { type Mounted, mount } from "pele/lazy"
import { onCleanup, onSettled } from "solid-js"
import { css } from "../../../styled-system/css"

// 図の色と書体は、本文と同じトークンから取る。明暗が替わっても描き直さない（pele は CSS 変数で色を持つ）
const diagram = css({
  "--pele-bg": "token(colors.surface)",
  "--pele-fg": "token(colors.fg)",
  "--pele-muted": "token(colors.fg.muted)",
  "--pele-line": "token(colors.fg.muted)",
  "--pele-surface": "token(colors.fill)",
  "--pele-surface-alt": "color-mix(in srgb, token(colors.fg) 14%, transparent)",
  "--pele-border": "token(colors.fg.muted)",
  "--pele-accent": "token(colors.accent)",
  "--pele-font": "token(fonts.text)",
  "--pele-font-mono": "token(fonts.mono)",
  "--pele-radius": "token(radii.inline)",
  // 大きい図は、pele が島の幅に縮めて出す（拡大の手は出さない）。島より狭い図は中央に置く
  maxWidth: "full",
  overflow: "hidden",
  "& > svg": { display: "block", marginInline: "auto" },
})

export type MermaidDiagramProps = {
  /** 図の元の文字（```mermaid の中身） */
  source: string
  /** 図の中のリンクを押した。書いてある先（href）を渡す。開くかどうかは受け取った側が決める */
  onLink: (href: string) => void
}

/**
 * mermaid の図。pele が島の幅に合わせて SVG を描く。図の種類は markdownToHast が読み終えているので、その場で描ける。
 * 書き損じは、pele がその場にエラーを文字で出す
 */
export function MermaidDiagram(props: MermaidDiagramProps) {
  let element: HTMLDivElement | undefined
  let mounted: Mounted | undefined

  // 島に置かれてから描く（幅と書体を測る）
  onSettled(() => {
    if (element) mounted = mount(element, props.source, { zoom: false, onError: () => {} })
  })
  onCleanup(() => mounted?.destroy())

  return (
    <div
      ref={element}
      class={diagram}
      onClick={(event: MouseEvent) => {
        const link = (event.target as Element).closest?.("a")
        if (!link) return
        // 図の中のリンクも webview の中では移らない
        event.preventDefault()
        const href = link.getAttribute("href") ?? link.getAttribute("xlink:href")
        if (href) props.onLink(href)
      }}
    />
  )
}
