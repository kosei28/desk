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

/** 図をその場で描く手（図の要素から引く）。描いた図は、今の幅で組み直す */
const layouts = new WeakMap<Element, () => void>()

/**
 * root の中の図の高さを、今の幅で決める。まだ描いていない図は描き、描いた図は幅が変わっていれば組み直す。
 * 図は島に置かれてから描き、幅が変わったら次のフレームで組み直す（隠れたタブの図は幅 0 のまま描いてある）ので、
 * 本文の中の位置を使う前（見出しへ送る前、タブの位置を戻す前）に呼ぶ。隠れている図は何もしない
 */
export function drawDiagrams(root: ParentNode) {
  for (const element of root.querySelectorAll("[data-mermaid]")) layouts.get(element)?.()
}

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

  const draw = () => {
    if (!element) return
    if (mounted) mounted.refresh()
    else mounted = mount(element, props.source, { zoom: false, onError: () => {} })
  }
  // 島に置かれてから描く（幅と書体を測る）。その前に drawDiagrams が描いていれば、幅が同じなら何もしない
  onSettled(draw)
  onCleanup(() => mounted?.destroy())

  return (
    <div
      ref={(el) => {
        element = el
        layouts.set(el, draw)
      }}
      data-mermaid
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
