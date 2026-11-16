import type { ResizableLayoutChangeDetails } from "@ui-sdk/solid/resizable"
import { createRoot, flush } from "solid-js"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { createColumns } from "./columns"

// jsdom には大きさも ResizeObserver も無い。列の入れ物と 3 つのパネルを作り、パネルの幅の合計（Resizable の 100%）を決めて、
// ResizeObserver の知らせを手で送る
let notify: (() => void) | undefined
let space = 0

beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(callback: () => void) {
        notify = callback
      }
      observe() {}
      disconnect() {}
    },
  )
})

afterEach(() => {
  vi.unstubAllGlobals()
  notify = undefined
})

function setup() {
  const root = document.createElement("div")
  for (let i = 0; i < 3; i++) {
    const panel = document.createElement("div")
    panel.dataset.slot = "resizable-panel"
    // どのパネルも合計の 1/3 として測る（合計だけが使われる）
    panel.getBoundingClientRect = () => ({ width: space / 3 }) as DOMRect
    root.append(panel)
  }
  const columns = createRoot(() => createColumns())
  columns.ref(root)
  return columns
}

/** 窓の幅が変わり、パネルの幅の合計が total になった */
function resizeTo(total: number) {
  space = total
  notify?.()
  flush()
}

const change = (columns: ReturnType<typeof createColumns>, next: number[], reason: ResizableLayoutChangeDetails["reason"]) => {
  columns.onLayoutChange(next, { reason } as ResizableLayoutChangeDetails)
  flush()
}

/** px の幅を、そのときの合計の割合にする */
const at = (widths: number[]) => widths.map((w) => (w / space) * 100)

/** 割合を、そのときの合計の px にする */
const pixels = (columns: ReturnType<typeof createColumns>) => columns.layout()?.map((p) => Math.round((p / 100) * space))

describe("createColumns", () => {
  it("窓の幅が変わっても、左右の列は既定の幅（px）のままで、真ん中だけが伸び縮みする", () => {
    const columns = setup()
    resizeTo(1424)
    expect(pixels(columns)).toEqual([292, 840, 292])
    resizeTo(1784)
    expect(pixels(columns)).toEqual([292, 1200, 292])
    resizeTo(1084)
    expect(pixels(columns)).toEqual([292, 500, 292])
  })

  it("真ん中が 300px を割るほど窓を狭めたら左右を縮め、広げ直したら元の幅に戻す", () => {
    const columns = setup()
    resizeTo(704)
    expect(pixels(columns)).toEqual([202, 300, 202])
    resizeTo(1424)
    expect(pixels(columns)).toEqual([292, 840, 292])
  })

  it("境目を動かした幅を覚え、そのあと窓の幅が変わっても保つ", () => {
    const columns = setup()
    resizeTo(1424)
    change(columns, at([332, 800, 292]), "drag")
    resizeTo(1784)
    expect(pixels(columns)).toEqual([332, 1160, 292])
  })

  it("Resizable が窓の幅に合わせ直した割合ではなく、覚えている幅から出し直す", () => {
    const columns = setup()
    resizeTo(1424)
    change(columns, [30, 40, 30], "resize")
    expect(pixels(columns)).toEqual([292, 840, 292])
  })

  // Resizable は真ん中が下限に当たると、もう片方の列まで押して縮める。その割合を渡す
  it("左の境目を広げても右の列は押されず、真ん中が下限に当たったら止まる。押されかけた右の幅は覚えない", () => {
    const columns = setup()
    resizeTo(1000)
    change(columns, at([442, 300, 258]), "drag")
    expect(pixels(columns)).toEqual([408, 300, 292])
    resizeTo(1400)
    expect(pixels(columns)).toEqual([408, 700, 292])
  })

  it("右の境目をキーで広げても左の列は押されない", () => {
    const columns = setup()
    resizeTo(1000)
    change(columns, at([250, 300, 450]), "step-key")
    expect(pixels(columns)).toEqual([292, 300, 408])
  })

  it("同じ窓の幅でも、もう片方の列の幅で動かせる範囲が変わる", () => {
    const columns = setup()
    resizeTo(1000)
    change(columns, at([292, 308, 400]), "drag")
    expect(pixels(columns)).toEqual([292, 308, 400])
    change(columns, at([480, 300, 220]), "drag")
    expect(pixels(columns)).toEqual([300, 300, 400])
    change(columns, at([300, 500, 200]), "drag")
    change(columns, at([480, 300, 220]), "drag")
    expect(pixels(columns)).toEqual([480, 320, 200])
  })

  it("窓が狭くて動けなかったときは、縮めて出している幅を覚え直さない", () => {
    const columns = setup()
    resizeTo(704)
    change(columns, at([210, 300, 194]), "drag")
    expect(pixels(columns)).toEqual([202, 300, 202])
    resizeTo(1424)
    expect(pixels(columns)).toEqual([292, 840, 292])
  })

  it("既定の幅に戻した列は、その幅を人が決めた幅として覚え、もう片方の列は動かさない", () => {
    const columns = setup()
    resizeTo(1424)
    change(columns, at([400, 732, 292]), "drag")
    change(columns, at([400, 552, 472]), "drag")
    columns.reset("left")
    flush()
    expect(pixels(columns)).toEqual([292, 660, 472])
    columns.reset("right")
    flush()
    expect(pixels(columns)).toEqual([292, 840, 292])
    resizeTo(1784)
    expect(pixels(columns)).toEqual([292, 1200, 292])
  })

  it("窓が狭くて既定の幅が入らなければ、入るところまでで出し、広げたら既定の幅で出す", () => {
    const columns = setup()
    resizeTo(900)
    change(columns, at([200, 408, 292]), "drag")
    change(columns, at([200, 300, 400]), "drag")
    columns.reset("left")
    flush()
    expect(pixels(columns)).toEqual([200, 300, 400])
    resizeTo(1400)
    expect(pixels(columns)).toEqual([292, 708, 400])
  })

  it("左右の列の既定の幅はそろっていて、トークンの幅から列の間の余白を除いた幅", () => {
    const columns = setup()
    expect(columns.left.defaultSize).toBe("292px")
    expect(columns.right.defaultSize).toBe("292px")
  })

  it("左右の列の最小・最大はそろっていて、真ん中の下限は 300px", () => {
    const columns = setup()
    expect([columns.left.minSize, columns.left.maxSize]).toEqual([columns.right.minSize, columns.right.maxSize])
    expect(columns.main.minSize).toBe("300px")
  })
})
