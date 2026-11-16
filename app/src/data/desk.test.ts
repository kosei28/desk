import { emit } from "@tauri-apps/api/event"
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks"
import { createRoot, flush } from "solid-js"
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
    await vi.waitFor(() => expect(desk.loaded()).toBe(true))
    expect(desk.index()).toBeNull()
    expect(desk.error()).toBeNull()
  })

  it("開いているツリーの索引をもらい、今のタブのファイルを引く", async () => {
    const calls = backend({ open: workbench })
    const desk = open()
    expect(desk.loaded()).toBe(false)
    await vi.waitFor(() => expect(desk.index()).toEqual(workbench))
    expect(desk.loaded()).toBe(true)
    expect(calls.filter((c) => c === "index")).toHaveLength(1)

    desk.open("projects/recipe-app/README.md")
    flush()
    await vi.waitFor(() => expect(desk.selectedDoc()?.title).toBe("献立プランナー"))
    expect(desk.tabs()).toEqual(["projects/recipe-app/README.md"])
  })

  it("開くといつも新しいタブで開き、今のタブの右に置いて切り替える", async () => {
    backend({ open: workbench })
    const desk = open()
    await vi.waitFor(() => expect(desk.index()).toEqual(workbench))
    const [a, b, c] = ["README.md", "projects/recipe-app/README.md", "projects/recipe-app/tasks/2026-11-09-printable-menu.md"]

    desk.open(a)
    flush()
    desk.open(b)
    flush()
    expect(desk.tabs()).toEqual([a, b])
    expect(desk.selected()).toBe(b)

    desk.open(a)
    flush()
    desk.open(c)
    flush()
    expect(desk.tabs()).toEqual([a, c, b])
    expect(desk.selected()).toBe(c)
  })

  it("もう開いているファイルを開くと、タブを足さずにそのタブに切り替える", async () => {
    backend({ open: workbench })
    const desk = open()
    await vi.waitFor(() => expect(desk.index()).toEqual(workbench))
    desk.open("README.md")
    flush()
    desk.open("projects/recipe-app/README.md")
    flush()

    desk.open("README.md")
    flush()
    expect(desk.tabs()).toEqual(["README.md", "projects/recipe-app/README.md"])
    expect(desk.selected()).toBe("README.md")
    desk.open("projects/recipe-app/README.md")
    flush()
    expect(desk.tabs()).toEqual(["README.md", "projects/recipe-app/README.md"])
    expect(desk.selected()).toBe("projects/recipe-app/README.md")
  })

  it("今のタブを閉じると右隣、右端なら左隣が今のタブになる。最後のタブを閉じると、今のタブは無い", async () => {
    backend({ open: workbench })
    const desk = open()
    await vi.waitFor(() => expect(desk.index()).toEqual(workbench))
    const [a, b] = ["README.md", "projects/recipe-app/README.md"]
    desk.open(a)
    flush()
    desk.open(b)
    flush()
    desk.open(a)
    flush()

    desk.close(a)
    flush()
    expect(desk.selected()).toBe(b)
    desk.open(a)
    flush()
    desk.close(a)
    flush()
    expect(desk.selected()).toBe(b)

    desk.open(a)
    flush()
    desk.close(b)
    flush()
    expect(desk.tabs()).toEqual([a])
    expect(desk.selected()).toBe(a)
    desk.close(a)
    flush()
    expect(desk.tabs()).toEqual([])
    expect(desk.selected()).toBeNull()
    expect(desk.selectedDoc()).toBeUndefined()
  })

  it("今でないタブを閉じても、今のタブは変わらない", async () => {
    backend({ open: workbench })
    const desk = open()
    await vi.waitFor(() => expect(desk.index()).toEqual(workbench))
    desk.open("README.md")
    flush()
    desk.open("projects/recipe-app/README.md")
    flush()
    desk.close("README.md")
    flush()
    expect(desk.selected()).toBe("projects/recipe-app/README.md")
  })

  it("タブを隣に切り替え、端からは反対の端に回る", async () => {
    backend({ open: workbench })
    const desk = open()
    await vi.waitFor(() => expect(desk.index()).toEqual(workbench))
    const [a, b] = ["README.md", "projects/recipe-app/README.md"]
    desk.cycle(1)
    flush()
    expect(desk.selected()).toBeNull()
    desk.open(a)
    flush()
    desk.open(b)
    flush()

    desk.cycle(1)
    flush()
    expect(desk.selected()).toBe(a)
    desk.cycle(-1)
    flush()
    expect(desk.selected()).toBe(b)
    desk.cycle(1)
    flush()
    expect(desk.selected()).toBe(a)
  })

  it("ツリーが変わったら索引をもらい直し、タブは残す。消えたファイルのタブは閉じる", async () => {
    const state = { open: workbench as Index | null }
    backend(state)
    const desk = open()
    await vi.waitFor(() => expect(desk.index()).toEqual(workbench))
    desk.open("README.md")
    flush()
    desk.open("projects/recipe-app/README.md")
    flush()
    desk.open("README.md")
    flush()

    const renamed = { ...workbench, docs: [doc("README.md", "机"), workbench.docs[1]] }
    state.open = renamed
    await emit("tree-changed")
    await vi.waitFor(() => expect(desk.selectedDoc()?.title).toBe("机"))

    state.open = { ...workbench, docs: [workbench.docs[1]] }
    await emit("tree-changed")
    await vi.waitFor(() => expect(desk.index()?.docs).toHaveLength(1))
    expect(desk.tabs()).toEqual(["projects/recipe-app/README.md"])
    expect(desk.selected()).toBe("projects/recipe-app/README.md")
  })

  it("フォルダーを選んで別のツリーを開くと、その索引をもらい、タブを全部閉じる", async () => {
    const calls = backend({ open: workbench, picked: garden })
    const desk = open()
    await vi.waitFor(() => expect(desk.index()).toEqual(workbench))
    desk.open("README.md")
    flush()

    await desk.choose()
    expect(calls).toContain("choose_root")
    expect(desk.index()).toEqual(garden)
    expect(desk.tabs()).toEqual([])
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
