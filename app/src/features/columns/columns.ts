import type { ResizableLayoutChangeDetails, ResizableSize } from "@ui-sdk/solid/resizable"
import { createSignal, onCleanup } from "solid-js"
import { token } from "../../../styled-system/tokens"

const px = (value: string) => Number.parseFloat(value)

/** 列の間の余白（ResizableHandle の幅。レシピ resizeHandle） */
const gap = px(token("spacing.2"))

// 左右の列の幅（トークンの sidebar・inspector）は列の間の余白 8px を含む。パネルはそれを除いた幅。最小・最大は左右でそろえる
const left = { default: px(token("sizes.sidebar")) - gap, min: 200, max: 480 }
const right = { default: px(token("sizes.inspector")) - gap, min: 200, max: 480 }
/** 真ん中の列（タブと島）の幅の下限。窓の最小の幅（720px）から列の間の 16px を除いた 704px に、左右の最小と合わせて収まる */
const mainMin = 300

const size = (value: number): ResizableSize => `${value}px`
const clamp = (value: number, range: { min: number; max: number }) => Math.min(range.max, Math.max(range.min, value))

/**
 * 3 列（左の列・真ん中の列・右の列）の幅。窓の幅が変わっても左右の列の幅（px）を保ち、真ん中の列だけを伸び縮みさせる。
 * 境目を動かしても、もう片方の列の幅は変えず、真ん中の列だけで受ける。動かせる範囲は、もう片方の幅と真ん中の下限で決まる。
 * 列は畳まない。境目をダブルクリックすると、その列を既定の幅に戻す。幅はアプリを閉じたら残さない（次に開いたときは既定の幅）。
 *
 * Resizable は幅を割合で持ち、窓の幅が変わっても割合が最小・最大に収まっていれば何も知らせない。
 * そこで列の入れ物の大きさを自分で見て、覚えている幅（px）から割合を出し直し、`layout` で渡す
 */
export function createColumns() {
  const [layout, setLayout] = createSignal<number[] | undefined>(undefined)
  // 人が決めた左右の列の幅（px）。境目を動かしたときと、既定の幅に戻したときだけ覚え直す
  let widths = { left: left.default, right: right.default }
  let root: HTMLElement | undefined
  let observer: ResizeObserver | undefined

  /** パネルの幅の合計（Resizable の割合の 100%） */
  function space() {
    let total = 0
    for (const panel of root?.querySelectorAll<HTMLElement>(":scope > [data-slot=resizable-panel]") ?? []) {
      total += panel.getBoundingClientRect().width
    }
    return total
  }

  /** 覚えている幅から割合を出す。真ん中の列が下限を割るなら、左右を最小の幅まで、余りの比で縮める */
  function fit(total: number): number[] {
    let l = clamp(widths.left, left)
    let r = clamp(widths.right, right)
    const over = mainMin - (total - l - r)
    if (over > 0) {
      const slackL = l - left.min
      const slackR = r - right.min
      const slack = slackL + slackR
      if (slack > 0) {
        const take = Math.min(over, slack)
        l -= (take * slackL) / slack
        r -= (take * slackR) / slack
      }
    }
    const pl = (l / total) * 100
    const pr = (r / total) * 100
    return [pl, 100 - pl - pr, pr]
  }

  function refit() {
    const total = space()
    if (total > 0) setLayout(fit(total))
  }

  function onLayoutChange(next: number[], details: ResizableLayoutChangeDetails) {
    const total = space()
    if (next.length !== 3 || total <= 0) {
      setLayout(next)
      return
    }
    // 人が境目を動かしたときだけ、動かした列の幅を覚える
    if (details.reason === "drag" || details.reason === "step-key") {
      const shown = layout() ?? fit(total)
      const grown = (i: number) => next[i] - shown[i]
      // Resizable は真ん中が下限に当たると、もう片方の列まで押して縮める。両方が動いたら、広がったほうが動かした列
      const side =
        Math.abs(grown(0)) < 1e-6 && Math.abs(grown(2)) < 1e-6
          ? undefined
          : Math.abs(grown(2)) < 1e-6 || (Math.abs(grown(0)) >= 1e-6 && grown(0) > 0)
            ? "left"
            : "right"
      if (side === undefined) return
      const before = (shown[side === "left" ? 0 : 2] / 100) * total
      const width = place(side, (next[side === "left" ? 0 : 2] / 100) * total, total)
      // 窓が狭くて動けなかったときは、縮めて出している幅を覚え直さない
      if (Math.abs(width - before) > 0.5) widths = { ...widths, [side]: width }
      return
    }
    // 窓の幅に合わせ直したときなどは、人が決めた幅から出し直す
    setLayout(fit(total))
  }

  /**
   * 片方の列を want の幅にして出し、出した幅を返す。もう片方の列は今の見た目の幅のまま動かさず、真ん中の列で受ける。
   * 真ん中が下限を割るなら、そこで止める
   */
  function place(side: "left" | "right", want: number, total: number) {
    const range = side === "left" ? left : right
    const shown = layout() ?? fit(total)
    const other = (shown[side === "left" ? 2 : 0] / 100) * total
    const width = clamp(want, { min: range.min, max: Math.max(range.min, Math.min(range.max, total - other - mainMin)) })
    const [l, r] = side === "left" ? [width, other] : [other, width]
    const pl = (l / total) * 100
    const pr = (r / total) * 100
    setLayout([pl, 100 - pl - pr, pr])
    return width
  }

  /**
   * 列を既定の幅に戻し、人が決めた幅として覚える。もう片方の列は動かさない。
   * 窓が狭くて入らなければ、入るところまでで出す（次に窓の幅が変わったら、覚えた幅から出し直す）
   */
  function reset(side: "left" | "right") {
    const width = side === "left" ? left.default : right.default
    widths = { ...widths, [side]: width }
    const total = space()
    if (total > 0) place(side, width, total)
  }

  onCleanup(() => observer?.disconnect())

  return {
    ref(el: HTMLElement) {
      root = el
      if (typeof ResizeObserver === "undefined") return
      observer?.disconnect()
      observer = new ResizeObserver(refit)
      observer.observe(el)
    },
    layout,
    onLayoutChange,
    reset,
    left: { defaultSize: size(left.default), minSize: size(left.min), maxSize: size(left.max) } satisfies Panel,
    main: { minSize: size(mainMin) } satisfies Panel,
    right: { defaultSize: size(right.default), minSize: size(right.min), maxSize: size(right.max) } satisfies Panel,
  }
}

type Panel = Record<string, ResizableSize>
