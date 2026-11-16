import { emit } from "@tauri-apps/api/event"
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks"
import { setPlatform } from "@ui-sdk/core/testing"
import { cleanup, render, screen, within } from "@solidjs/testing-library"
import { createAgent } from "@ui-sdk/solid/agent"
import { flush } from "solid-js"
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest"
import type { Doc, Index } from "./data/types"
import { click, keys } from "./test/input"

import { App } from "./app"

// 1 つのファイルに 1 つ。外した部品も offPage として覚えているので、作り直さない
const agent = createAgent()

const doc = (path: string, title: string, fields: Partial<Doc> = {}): Doc => ({
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
  ...fields,
})

// 架空のツリー
const workbench: Index = {
  root: "/trees/workbench",
  docs: [
    doc("README.md", "作業机"),
    doc("projects/recipe-app/README.md", "献立プランナー"),
    doc("projects/recipe-app/tasks/2026-11-09-printable-menu.md", "献立を印刷できるようにする", {
      kind: "task",
      status: "inprogress",
    }),
    doc("projects/recipe-app/tasks/2026-11-10-decide-paper-size.md", "用紙の大きさを決める", {
      kind: "task",
      status: "todo",
      needs_human: true,
    }),
    doc("projects/recipe-app/archive/tasks/2026-10-01-old.md", "古いタスク", { kind: "task", status: "done" }),
  ],
}

/** Rust の側の代わり。呼ばれたコマンドを覚える */
function backend(state: { open: Index | null; picked?: Index }) {
  const calls: string[] = []
  mockIPC(
    (cmd) => {
      if (!cmd.startsWith("plugin:event|")) calls.push(cmd)
      if (cmd === "index") return state.open
      if (cmd === "choose_root") {
        if (state.picked) state.open = state.picked
        return null
      }
    },
    { shouldMockEvents: true },
  )
  return calls
}

const tick = () => new Promise((resolve) => setTimeout(resolve, 0))

async function renderApp(state: { open: Index | null; picked?: Index } = { open: workbench }) {
  const calls = backend(state)
  const result = render(() => <App />)
  await vi.waitFor(() => expect(calls).toContain("index"))
  await tick()
  flush()
  return Object.assign(result, { calls })
}

const surface = () => screen.getByRole("article")
const tree = () => screen.getByRole("tree", { name: "ファイル" })
const tabList = () => screen.getByRole("tablist", { name: "開いているファイル" })
const tabNames = () => within(tabList()).queryAllByRole("tab").map((tab) => tab.textContent)
const selectedTab = () => within(tabList()).queryAllByRole("tab").find((tab) => tab.getAttribute("aria-selected") === "true")
const rowOf = (text: string, nth = 0) =>
  within(tree()).getAllByText(text)[nth].closest<HTMLElement>("[data-slot=tree-label]")!
const selectedRows = () => [...tree().querySelectorAll("[data-slot=tree-label][data-selected]")].map((row) => row.textContent)
const heading = () => screen.queryByRole("heading", { level: 1 })?.textContent
const mouse = (target: Element, type: "mousedown" | "mouseup", detail: number) =>
  target.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, detail, button: 0 }))
const press = (key: string, modifiers: KeyboardEventInit = {}) => {
  const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...modifiers })
  ;(document.activeElement ?? document.body).dispatchEvent(event)
  flush()
  return event
}
async function metaClick(target: HTMLElement) {
  const meta = vi.spyOn(MouseEvent.prototype, "metaKey", "get").mockReturnValue(true)
  try {
    await click(target)
  } finally {
    meta.mockRestore()
  }
}
async function openFolders(...names: string[]) {
  for (const name of names) await click(rowOf(name))
}

beforeAll(() => setPlatform("apple"))

afterEach(async () => {
  cleanup()
  // 聞くのをやめる処理（非同期）が、偽の Tauri を消す前に走るように待つ
  await tick()
  clearMocks()
})

describe("App", () => {
  it("ツリーを開いていなければ、島でフォルダーを選べる。選ぶとそのツリーを出す", async () => {
    const { calls } = await renderApp({ open: null, picked: workbench })
    expect(screen.queryByRole("tree")).toBeNull()
    await click(screen.getByRole("button", { name: "desk のツリーのフォルダーを選ぶ" }))
    expect(calls).toContain("choose_root")
    await vi.waitFor(() => expect(screen.queryByRole("tree", { name: "ファイル" })).not.toBeNull())
    expect(surface().textContent).toBe("ファイルを選ぶと、ここに開く")
  })

  it("左の列はルートの中身から、README → フォルダーの順に出す。はじめはフォルダーを閉じ、島はファイルを選ぶよう出す", async () => {
    await renderApp()
    const rows = within(tree()).getAllByRole("treeitem").map((item) => item.getAttribute("aria-label") ?? item.textContent)
    expect(rows).toEqual(["README", "projects"])
    expect(surface().textContent).toBe("ファイルを選ぶと、ここに開く")
    expect(tabNames()).toEqual([])
  })

  it("行を押すといつも新しいタブで開き、今のタブの右に置く。もう開いているファイルは、そのタブに切り替える", async () => {
    await renderApp()
    await click(rowOf("README"))
    expect(tabNames()).toEqual(["作業机"])
    expect(heading()).toBe("作業机")
    expect(surface().textContent).not.toContain("README.md")

    await openFolders("projects", "recipe-app")
    await click(rowOf("README", 1))
    expect(tabNames()).toEqual(["作業机", "献立プランナー"])
    expect(heading()).toBe("献立プランナー")

    await click(rowOf("README"))
    expect(tabNames()).toEqual(["作業机", "献立プランナー"])
    expect(selectedTab()?.textContent).toBe("作業机")

    await openFolders("tasks")
    await click(rowOf("用紙の大きさを決める"))
    expect(tabNames()).toEqual(["作業机", "用紙の大きさを決める", "献立プランナー"])
    expect(selectedTab()?.textContent).toBe("用紙の大きさを決める")
    expect(heading()).toBe("用紙の大きさを決める")
  })

  it("⌘ を押しながら行を押しても、押さないときと同じに新しいタブで開く", async () => {
    await renderApp()
    await click(rowOf("README"))
    await openFolders("projects", "recipe-app", "tasks")
    await metaClick(rowOf("用紙の大きさを決める"))
    expect(tabNames()).toEqual(["作業机", "用紙の大きさを決める"])
    expect(selectedTab()?.textContent).toBe("用紙の大きさを決める")
  })

  it("タスクのタブには status の印を付け、人の印は付けない", async () => {
    await renderApp()
    await openFolders("projects", "recipe-app", "tasks")
    await click(rowOf("用紙の大きさを決める"))
    const tab = selectedTab()!
    expect(tab.querySelector("[aria-label=todo]")).not.toBeNull()
    expect(rowOf("用紙の大きさを決める").querySelector("[aria-label=人が要る]")).not.toBeNull()
    expect(tab.closest("[data-path]")!.querySelector("[aria-label=人が要る]")).toBeNull()
  })

  it("タブを押すと切り替わり、左の列の選択もついてきて祖先を開く", async () => {
    await renderApp()
    await openFolders("projects", "recipe-app", "tasks")
    await click(rowOf("献立を印刷できるようにする"))
    await click(rowOf("tasks"))
    await click(rowOf("recipe-app"))
    await click(rowOf("projects"))
    await click(rowOf("README"))
    expect(selectedRows()).toEqual(["README"])
    expect(within(tree()).queryByText("献立を印刷できるようにする")).toBeNull()

    await click(within(tabList()).getByRole("tab", { name: /献立を印刷できるようにする/ }))
    flush()
    await vi.waitFor(() => expect(selectedRows()).toEqual(["献立を印刷できるようにする"]))
    expect(heading()).toBe("献立を印刷できるようにする")
  })

  it("× でタブを閉じる。最後のタブを閉じると島は空になる", async () => {
    await renderApp()
    await click(rowOf("README"))
    await openFolders("projects", "recipe-app")
    await click(rowOf("README", 1))
    expect(tabNames()).toEqual(["作業机", "献立プランナー"])

    await click(screen.getByRole("button", { name: "献立プランナー を閉じる" }))
    expect(tabNames()).toEqual(["作業机"])
    expect(heading()).toBe("作業机")
    expect(selectedRows()).toEqual(["README"])

    await click(screen.getByRole("button", { name: "作業机 を閉じる" }))
    expect(tabNames()).toEqual([])
    expect(surface().textContent).toBe("ファイルを選ぶと、ここに開く")
    expect(selectedRows()).toEqual([])
  })

  it("⌘W で今のタブを閉じる。タブが無くても ⌘W を受け取る（窓を閉じるメニューに渡さない）", async () => {
    await renderApp()
    await click(rowOf("README"))
    await openFolders("projects", "recipe-app")
    await click(rowOf("README", 1))

    expect(press("w", { metaKey: true }).defaultPrevented).toBe(true)
    expect(tabNames()).toEqual(["作業机"])
    press("w", { metaKey: true })
    expect(tabNames()).toEqual([])
    expect(press("w", { metaKey: true }).defaultPrevented).toBe(true)
  })

  it("⌃Tab で右のタブ、⌃⇧Tab で左のタブに切り替え、端からは反対の端に回る", async () => {
    await renderApp()
    await click(rowOf("README"))
    await openFolders("projects", "recipe-app")
    await click(rowOf("README", 1))
    expect(selectedTab()?.textContent).toBe("献立プランナー")

    press("Tab", { ctrlKey: true })
    expect(selectedTab()?.textContent).toBe("作業机")
    press("Tab", { ctrlKey: true, shiftKey: true })
    expect(selectedTab()?.textContent).toBe("献立プランナー")
    press("Tab", { ctrlKey: true, shiftKey: true })
    expect(selectedTab()?.textContent).toBe("作業机")
  })

  it("同じツリーで索引をもらい直しても左の列を作り直さず、開いたフォルダーとフォーカスを保つ", async () => {
    const state = { open: workbench as Index | null }
    await renderApp(state)
    await openFolders("projects", "recipe-app")
    const before = tree()
    const item = (text: string) => rowOf(text).closest<HTMLElement>("[role=treeitem]")!
    item("recipe-app").focus()
    state.open = { ...workbench, docs: [...workbench.docs, doc("projects/recipe-app/inbox/2026-11-17-note.md", "メモ")] }
    await emit("tree-changed")
    await vi.waitFor(() => expect(within(tree()).queryByText("inbox")).not.toBeNull())
    expect(tree()).toBe(before)
    expect(item("projects").getAttribute("aria-expanded")).toBe("true")
    expect(item("recipe-app").getAttribute("aria-expanded")).toBe("true")
    expect(document.activeElement).toBe(item("recipe-app"))
  })

  it("ツリーが変わって今のタブのファイルが消えたら、そのタブを閉じる", async () => {
    const state = { open: workbench as Index | null }
    await renderApp(state)
    await click(rowOf("README"))
    state.open = { ...workbench, docs: workbench.docs.slice(1) }
    await emit("tree-changed")
    await vi.waitFor(() => expect(tabNames()).toEqual([]))
    expect(surface().textContent).toBe("ファイルを選ぶと、ここに開く")
  })

  // 帯の高さの中で何にも覆われていない所がどこも帯に当たるかは、レイアウトの話で jsdom では見られない（ヘッドレスの
  // Chromium で見る）。ここでは、窓を動かすのが帯の層だけで、島・行・タブ・列の入れ物では動かないことを見る
  it("帯の層を押すと窓を動かし、ダブルクリックでタイトルバーのアクションを送る。島・行・タブ・列の入れ物では何もしない", async () => {
    const { calls } = await renderApp()
    await click(rowOf("README"))
    const band = document.querySelector<HTMLElement>("[data-titlebar]")!
    const tab = selectedTab()!
    const doubleClick = (target: Element) => {
      mouse(target, "mousedown", 1)
      mouse(target, "mouseup", 1)
      mouse(target, "mousedown", 2)
      mouse(target, "mouseup", 2)
    }
    calls.length = 0

    for (const target of [surface(), rowOf("README"), tab, tabList(), surface().parentElement!, tree().closest("nav")!])
      doubleClick(target)
    await tick()
    expect(calls).toEqual([])

    doubleClick(band)
    await tick()
    expect(calls).toEqual(["plugin:window|start_dragging", "titlebar_double_click"])
  })

  it("エージェントが、名前でツリーとタブを見つけ、ファイルを開ける。フォルダーは開けない", async () => {
    await renderApp()
    const components = agent.snapshot().filter((entry) => !entry.offPage && entry.component !== "more")
    for (const entry of components) {
      expect(entry.label, `${entry.component} ${entry.ref} has no name`).toBeTruthy()
    }
    const fileTree = components.find((entry) => entry.label === "ファイル")
    expect(fileTree?.component).toBe("tree")
    // 値は自分のデータからではなく、エージェントが見るもの（snapshot）から取る（UI SDK の skill の agent のテスト）
    const items = (fileTree as { items?: { value: string; label: string }[] }).items ?? []
    const file = items.find((item) => item.label === "README")
    const folder = items.find((item) => item.label === "projects")
    expect(file?.value).toBe("README.md")
    expect(folder).toBeDefined()

    const opened = await agent.act({ steps: [{ ref: "ファイル", action: "setValue", args: [file!.value] }] })
    expect(opened.ok, opened.error?.message).toBe(true)
    flush()
    expect(heading()).toBe("作業机")
    expect(tabNames()).toEqual(["作業机"])

    await agent.act({ steps: [{ ref: "ファイル", action: "setValue", args: [folder!.value] }] })
    flush()
    expect(heading()).toBe("作業机")
  })
})

// jsdom には大きさが無い。3 つのパネルの幅の合計（Resizable の 100%）を決め、ResizeObserver の知らせを手で送る
describe("App の列の幅", () => {
  let observers: (() => void)[] = []
  let space = 0

  beforeEach(() => {
    observers = []
    vi.stubGlobal(
      "ResizeObserver",
      class {
        constructor(callback: () => void) {
          observers.push(callback)
        }
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    )
    const rect = HTMLElement.prototype.getBoundingClientRect
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      if (this.dataset.slot === "resizable-panel") return { width: space / 3, height: 900 } as DOMRect
      return rect.call(this)
    })
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  const panels = () => [...document.querySelectorAll<HTMLElement>("[data-slot=resizable-panel]")]
  /** 左・真ん中・右の幅（px）。Resizable がパネルに書く割合から出す */
  const widths = () =>
    panels().map((panel) => Math.round((Number.parseFloat(panel.style.getPropertyValue("--resizable-panel-size")) / 100) * space))
  const handle = (name: string) => screen.getByRole("separator", { name })
  const frame = () => new Promise((resolve) => requestAnimationFrame(resolve))

  async function resizeTo(total: number) {
    space = total
    for (const notify of observers) notify()
    await frame()
    await tick()
    flush()
  }

  async function renderColumns(total: number) {
    await renderApp()
    await resizeTo(total)
    expect(widths()).toEqual([292, total - 584, 292])
  }

  const doubleClick = (target: Element) => {
    target.dispatchEvent(new MouseEvent("dblclick", { bubbles: true, cancelable: true, detail: 2 }))
    flush()
  }

  it("境目をダブルクリックすると、その列を既定の幅に戻し、窓の幅が変わっても保つ", async () => {
    await renderColumns(1424)
    await click(handle("左の列の幅"))
    await keys("Shift+ArrowRight")
    await click(handle("右の列の幅"))
    await keys("Shift+ArrowLeft")
    flush()
    const [l, , r] = widths()
    expect(l).toBeGreaterThan(292)
    expect(r).toBeGreaterThan(292)

    doubleClick(handle("左の列の幅"))
    expect(widths()).toEqual([292, 1424 - 292 - r, r])
    doubleClick(handle("右の列の幅"))
    expect(widths()).toEqual([292, 840, 292])
    await resizeTo(1784)
    expect(widths()).toEqual([292, 1200, 292])
  })

  it("窓が狭いときにダブルクリックしても真ん中は 300px より狭くせず、広げたら既定の幅になる", async () => {
    await renderColumns(1424)
    await click(handle("左の列の幅"))
    await keys("Shift+ArrowRight", "Shift+ArrowRight")
    await resizeTo(704)
    doubleClick(handle("左の列の幅"))
    expect(widths()[1]).toBe(300)
    await resizeTo(1424)
    expect(widths()).toEqual([292, 840, 292])
  })

  it("Enter・Home・End・矢印のキーでも、列は畳まれず最小の幅で止まる", async () => {
    await renderColumns(1424)
    await click(handle("左の列の幅"))
    await keys("Enter", "Home")
    expect(widths()[0]).toBe(200)
    await keys("ArrowLeft", "ArrowLeft")
    expect(widths()[0]).toBe(200)
    await click(handle("右の列の幅"))
    await keys("Enter", "End")
    expect(widths()[2]).toBe(200)
    await keys("ArrowRight", "ArrowRight")
    expect(widths()[2]).toBe(200)
    expect(panels().filter((panel) => panel.hasAttribute("data-collapsed"))).toEqual([])
  })

  it("窓が狭くて左右を縮めているとき、境目をキーで動かしても反対の列は押されず、畳まれない", async () => {
    await renderColumns(1424)
    await resizeTo(704)
    expect(widths()).toEqual([202, 300, 202])
    await click(handle("左の列の幅"))
    await keys("ArrowRight", "ArrowRight", "ArrowRight")
    expect(widths()).toEqual([202, 300, 202])
    expect(panels().filter((panel) => panel.hasAttribute("data-collapsed"))).toEqual([])
  })

  it("境目を動かしても反対の列は変わらず、動かせる範囲は反対の列の幅で決まる", async () => {
    await renderColumns(1000)
    await click(handle("左の列の幅"))
    await keys("End")
    expect(widths()).toEqual([408, 300, 292])
    await click(handle("右の列の幅"))
    await keys("End")
    expect(widths()).toEqual([408, 392, 200])
    await click(handle("左の列の幅"))
    await keys("End")
    expect(widths()).toEqual([480, 320, 200])
  })
})
