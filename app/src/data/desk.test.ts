import { emit } from "@tauri-apps/api/event"
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks"
import { createRoot } from "solid-js"
import { afterEach, describe, expect, it, vi } from "vitest"
import { createDesk, type Desk } from "./desk"
import type { Doc, Index } from "./types"

const doc = (path: string, title: string): Doc => ({
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
  title,
  description: "",
  now: "",
  status: "",
  needs_human: false,
  headings: [],
  links: [],
})

const workbench: Index = {
  root: "/trees/workbench",
  docs: [doc("README.md", "作業机"), doc("projects/recipe-app/README.md", "献立プランナー")],
}
const garden: Index = { root: "/trees/garden", docs: [doc("README.md", "庭")] }

/** Rust の側の代わり。open は今開いているツリー、choose で開くツリーは picked */
function backend(state: { open: Index | null; picked?: Index | Error }) {
  const calls: string[] = []
  mockIPC(
    (cmd) => {
      calls.push(cmd)
      if (cmd === "index") return state.open
      if (cmd === "choose_root") {
        if (state.picked instanceof Error) throw state.picked.message
        if (state.picked) state.open = state.picked
        return null
      }
    },
    { shouldMockEvents: true },
  )
  return calls
}

let dispose: () => void = () => {}
function open(): Desk {
  return createRoot((d) => {
    dispose = d
    return createDesk()
  })
}

const tick = () => new Promise((resolve) => setTimeout(resolve, 0))

afterEach(async () => {
  dispose()
  // 聞くのをやめる処理（非同期）が、偽の Tauri を消す前に走るように待つ
  await tick()
  clearMocks()
})

describe("createDesk", () => {
  it("ツリーを開いていなければ、索引は null のまま", async () => {
    const calls = backend({ open: null })
    const desk = open()
    await vi.waitFor(() => expect(calls).toContain("index"))
    expect(desk.index()).toBeNull()
    expect(desk.error()).toBeNull()
  })

  it("開いているツリーの索引をもらい、選んだパスのファイルを引く", async () => {
    backend({ open: workbench })
    const desk = open()
    await vi.waitFor(() => expect(desk.index()).toEqual(workbench))

    desk.select("projects/recipe-app/README.md")
    await vi.waitFor(() => expect(desk.selectedDoc()?.title).toBe("献立プランナー"))
    desk.select(null)
    await vi.waitFor(() => expect(desk.selectedDoc()).toBeUndefined())
  })

  it("ツリーが変わったら索引をもらい直し、選択は残す。選んだファイルが消えたら、選んだファイルは無い", async () => {
    const state = { open: workbench as Index | null }
    backend(state)
    const desk = open()
    await vi.waitFor(() => expect(desk.index()).toEqual(workbench))
    desk.select("README.md")

    const renamed = { ...workbench, docs: [doc("README.md", "机"), workbench.docs[1]] }
    state.open = renamed
    await emit("tree-changed")
    await vi.waitFor(() => expect(desk.selectedDoc()?.title).toBe("机"))

    state.open = { ...workbench, docs: [workbench.docs[1]] }
    await emit("tree-changed")
    await vi.waitFor(() => expect(desk.index()?.docs).toHaveLength(1))
    expect(desk.selected()).toBe("README.md")
    expect(desk.selectedDoc()).toBeUndefined()
  })

  it("フォルダーを選んで別のツリーを開くと、その索引をもらい、選択を外す", async () => {
    const calls = backend({ open: workbench, picked: garden })
    const desk = open()
    await vi.waitFor(() => expect(desk.index()).toEqual(workbench))
    desk.select("README.md")

    await desk.choose()
    expect(calls).toContain("choose_root")
    expect(desk.index()).toEqual(garden)
    expect(desk.selected()).toBeNull()
  })

  it("ツリーを開けなければ、前の索引のまま説明を出す", async () => {
    backend({ open: workbench, picked: new Error("/tmp は desk のツリーの中ではない") })
    const desk = open()
    await vi.waitFor(() => expect(desk.index()).toEqual(workbench))

    await desk.choose()
    expect(desk.error()).toBe("/tmp は desk のツリーの中ではない")
    expect(desk.index()).toEqual(workbench)
  })

  it("読み込みが重なったら、先に頼んだものの結果が後から返っても使わない", async () => {
    const answers: ((index: Index) => void)[] = []
    mockIPC(
      (cmd) => {
        if (cmd === "index") return new Promise<Index>((resolve) => answers.push(resolve))
      },
      { shouldMockEvents: true },
    )
    const desk = open()
    await vi.waitFor(() => expect(answers).toHaveLength(1))
    await emit("tree-changed")
    await vi.waitFor(() => expect(answers).toHaveLength(2))

    answers[1](garden)
    await vi.waitFor(() => expect(desk.index()).toEqual(garden))
    answers[0](workbench)
    await tick()
    expect(desk.index()).toEqual(garden)
  })

  it("owner が消えたら、ツリーが変わっても索引をもらわない", async () => {
    const calls = backend({ open: workbench })
    const indexCalls = () => calls.filter((c) => c === "index").length
    open()
    await vi.waitFor(() => expect(indexCalls()).toBe(1))
    await emit("tree-changed")
    await vi.waitFor(() => expect(indexCalls()).toBe(2))

    dispose()
    await tick()
    await emit("tree-changed")
    await tick()
    expect(indexCalls()).toBe(2)
  })
})
