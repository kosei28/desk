import { clearMocks, mockIPC } from "@tauri-apps/api/mocks"
import { cleanup, render } from "@solidjs/testing-library"
import { flush } from "solid-js"
import { afterEach, describe, expect, it, vi } from "vitest"
import type { Doc } from "../../data/types"
import { FilePreview } from "./file-preview"

const doc = (path: string, fields: Partial<Doc> = {}): Doc => ({
  path,
  kind: null,
  project: "",
  archived: false,
  project_archived: false,
  asset: false,
  problem: null,
  is_md: true,
  frontmatter: null,
  frontmatter_error: null,
  title: "",
  description: "",
  now: "",
  status: "",
  needs_human: false,
  base: "",
  headings: [],
  links: [],
  ...fields,
})

// 架空のツリー
const LAYOUT = "projects/recipe-app/artifacts/print-layout.md"
const SAMPLE = "projects/recipe-app/artifacts/assets/print-sample.png"

afterEach(() => {
  cleanup()
  clearMocks()
  vi.restoreAllMocks()
})

describe("FilePreview の anchor", () => {
  /**
   * 本文のコマンドを、release を呼ぶまで返さない（fail なら断る）ようにして、島（縦にスクロールする箱）の中に描く。
   * 島の上の端は 20、内側の余白は 48、位置は 100、見出し「余白」は 520 にある
   */
  function showInIsland(target: Doc, body: string, options: { fail?: boolean } = {}) {
    let release!: () => void
    const held = new Promise<void>((resolve) => (release = resolve))
    mockIPC(async (cmd) => {
      if (cmd !== "body") return
      await held
      if (options.fail) throw new Error("読めない")
      return body
    })
    const onAnchorDone = vi.fn()
    // 描いた時点で送ることがあるので、島の位置と scrollTo は描く前に用意する
    const scrolls: unknown[] = []
    let island: HTMLDivElement | undefined
    vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
      return { top: this === island ? 20 : this.id === "user-content-余白" ? 520 : 0 } as DOMRect
    })
    const hold = (element: HTMLDivElement) => {
      island = element
      element.scrollTo = ((options: unknown) => scrolls.push(options)) as never
      element.scrollTop = 100
    }
    render(() => (
      <div ref={hold} style={{ "overflow-y": "auto", "padding-top": "48px" }}>
        <FilePreview
          doc={target}
          title="印刷のレイアウト"
          byPath={new Map([[target.path, target]])}
          onOpen={() => {}}
          onOpenUrl={() => {}}
          anchor={{ id: "user-content-余白" }}
          onAnchorDone={onAnchorDone}
        />
      </div>
    ))
    const arrive = async () => {
      release()
      // 本文が届くのを待つ（届いた後の描き直しと送るのは、同じ流れの中）
      await vi.waitFor(() => expect(onAnchorDone).toHaveBeenCalled())
    }
    return { scrolls, onAnchorDone, arrive }
  }
  const frame = () => new Promise((resolve) => requestAnimationFrame(resolve))

  it("本文を描くまで送らず、描いたら次のフレームを待たずにその見出しへ送り、知らせる", async () => {
    const { scrolls, onAnchorDone, arrive } = showInIsland(doc(LAYOUT), "# 印刷のレイアウト\n\n本文\n\n## 余白\n")
    flush()
    await frame()
    expect(scrolls).toEqual([])
    expect(onAnchorDone).not.toHaveBeenCalled()

    // フレームを止めても送れる（描いたのと同じ流れの中で送り、上から描いた絵を 1 枚も出さない）
    vi.spyOn(window, "requestAnimationFrame").mockImplementation(() => 0)
    await arrive()
    expect(scrolls).toEqual([{ top: 100 + 500 - 48 }])
    expect(onAnchorDone).toHaveBeenCalledOnce()
  })

  it("見出しが無ければ、上から出す", async () => {
    const { scrolls, onAnchorDone, arrive } = showInIsland(doc(LAYOUT), "# 印刷のレイアウト\n\n本文\n")
    await arrive()
    expect(scrolls).toEqual([{ top: 0 }])
    expect(onAnchorDone).toHaveBeenCalledOnce()
  })

  it("本文が空でも、読めなくても、上から出して終える", async () => {
    const empty = showInIsland(doc(LAYOUT), "")
    await empty.arrive()
    expect(empty.scrolls).toEqual([{ top: 0 }])
    cleanup()

    vi.spyOn(console, "error").mockImplementation(() => {})
    const failed = showInIsland(doc(LAYOUT), "", { fail: true })
    await failed.arrive()
    expect(failed.scrolls).toEqual([{ top: 0 }])
    expect(failed.onAnchorDone).toHaveBeenCalledOnce()
  })

  it("Markdown でないファイルは本文を待たず、すぐ上から出す", () => {
    const { scrolls, onAnchorDone } = showInIsland(doc(SAMPLE, { is_md: false, asset: true }), "")
    flush()
    expect(scrolls).toEqual([{ top: 0 }])
    expect(onAnchorDone).toHaveBeenCalledOnce()
  })
})
