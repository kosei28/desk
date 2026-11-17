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
  base: "",
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
type Backend = {
  open: Index | null
  picked?: Index
  /** 本文のコマンドが返す本文（ルートからのパス → 本文）。無いパスは空 */
  bodies?: Record<string, string>
  /** あれば、本文のコマンドはこれが済むまで返さない（webview に本文が届くまでの間を見る） */
  bodyHeld?: Promise<void>
}

function backend(state: Backend) {
  const calls: string[] = []
  /** 呼んだコマンドと、渡した引数 */
  const requests: { cmd: string; args: unknown }[] = []
  mockIPC(
    (cmd, args) => {
      if (!cmd.startsWith("plugin:event|")) {
        calls.push(cmd)
        requests.push({ cmd, args })
      }
      if (cmd === "index") return state.open
      if (cmd === "body") {
        const text = state.bodies?.[(args as { path: string }).path] ?? ""
        return state.bodyHeld ? state.bodyHeld.then(() => text) : text
      }
      if (cmd === "choose_root") {
        if (state.picked) state.open = state.picked
        return null
      }
    },
    { shouldMockEvents: true },
  )
  return { calls, requests }
}

const tick = () => new Promise((resolve) => setTimeout(resolve, 0))

async function renderApp(state: Backend = { open: workbench }) {
  const { calls, requests } = backend(state)
  const result = render(() => <App />)
  await vi.waitFor(() => expect(calls).toContain("index"))
  await tick()
  flush()
  return Object.assign(result, { calls, requests })
}

const surface = () => screen.getByRole("article")
/** 今のタブのパネル。ほかのタブのパネルも、隠して島の中に残る */
const shown = () => surface().querySelector<HTMLElement>("[role=tabpanel][data-selected]")!
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

// 右の列を見るための架空のツリー。印刷のタスクに、子タスク・関連ファイル・リンクをつなぐ
const MENU = "projects/recipe-app/tasks/2026-11-09-printable-menu.md"
const PAPER = "projects/recipe-app/tasks/2026-11-10-decide-paper-size.md"
const LAYOUT = "projects/recipe-app/artifacts/print-layout.md"
const relative = (key: string, target: string) => ({
  key,
  text: "",
  raw: target,
  line: 0,
  start: null,
  form: "relative" as const,
  target,
  anchor: "",
})
const linked: Index = {
  root: "/trees/workbench",
  docs: [
    doc("README.md", "作業机", { links: [relative("body", MENU)] }),
    doc("projects/recipe-app/README.md", "献立プランナー"),
    doc(MENU, "献立を印刷できるようにする", {
      kind: "task",
      status: "inprogress",
      frontmatter: [
        { key: "status", value: "inprogress" },
        { key: "needs_human", value: "false" },
        { key: "pr", value: "https://github.com/example/recipe-app/pull/128" },
      ],
      links: [relative("body", LAYOUT)],
    }),
    doc(PAPER, "用紙の大きさを決める", {
      kind: "task",
      status: "todo",
      needs_human: true,
      links: [relative("parent", MENU)],
    }),
    doc(LAYOUT, "印刷のレイアウト", { kind: "artifact", links: [relative("tasks", MENU)] }),
  ],
}

describe("App の右の列", () => {
  const column = () => screen.getByRole("complementary", { name: "今のファイル" })
  const columnRow = (title: string) => within(column()).getAllByText(title)[0].closest<HTMLElement>("a")!
  const sectionButton = (title: string) => within(column()).getByRole("button", { name: title })

  async function openMenu() {
    const result = await renderApp({ open: linked })
    await openFolders("projects", "recipe-app", "tasks")
    await click(rowOf("献立を印刷できるようにする"))
    return result
  }

  it("何も開いていなければ出さず、開くと今のファイルのパス・札・見出しを出す", async () => {
    await renderApp({ open: linked })
    expect(screen.queryByRole("complementary", { name: "今のファイル" })).toBeNull()
    await openFolders("projects", "recipe-app", "tasks")
    await click(rowOf("献立を印刷できるようにする"))
    expect(column().textContent).toContain(MENU)
    expect(within(column()).getByText("inprogress")).toBeTruthy()
    const sections = within(column())
      .getAllByRole("button")
      .map((button) => button.textContent)
    expect(sections).toEqual(["コード", "親子と依存", "関連ファイル", "リンク", "他からのリンク"])
    expect(within(column()).getByText("子タスク")).toBeTruthy()
    // 見出しも、エージェントが名前で見つけられる
    const components = agent.snapshot().filter((entry) => !entry.offPage && entry.component !== "more")
    for (const entry of components) {
      expect(entry.label, `${entry.component} ${entry.ref} has no name`).toBeTruthy()
    }
  })

  it("行を押すと新しいタブで今のタブの右に開き、もう開いているファイルの行ならそのタブに移る", async () => {
    await openMenu()
    expect(tabNames()).toEqual(["献立を印刷できるようにする"])
    await click(columnRow("用紙の大きさを決める"))
    expect(tabNames()).toEqual(["献立を印刷できるようにする", "用紙の大きさを決める"])
    expect(heading()).toBe("用紙の大きさを決める")

    await click(columnRow("献立を印刷できるようにする"))
    expect(tabNames()).toEqual(["献立を印刷できるようにする", "用紙の大きさを決める"])
    expect(selectedTab()?.textContent).toBe("献立を印刷できるようにする")
    expect(heading()).toBe("献立を印刷できるようにする")

    // ⌘ を押しながら押しても同じ
    await metaClick(columnRow("印刷のレイアウト"))
    expect(tabNames()).toEqual(["献立を印刷できるようにする", "印刷のレイアウト", "用紙の大きさを決める"])
    expect(heading()).toBe("印刷のレイアウト")
  })

  it("行で Enter か Space を押すと開き、列は送らない", async () => {
    await openMenu()
    columnRow("用紙の大きさを決める").focus()
    expect(press(" ").defaultPrevented).toBe(true)
    expect(tabNames()).toEqual(["献立を印刷できるようにする", "用紙の大きさを決める"])
    // 開いたタブは本文を描き終えてから見せる
    await vi.waitFor(() => expect(heading()).toBe("用紙の大きさを決める"))

    await click(within(tabList()).getAllByRole("tab")[0])
    columnRow("印刷のレイアウト").focus()
    expect(press("Enter").defaultPrevented).toBe(true)
    expect(tabNames()).toEqual(["献立を印刷できるようにする", "印刷のレイアウト", "用紙の大きさを決める"])
    await vi.waitFor(() => expect(heading()).toBe("印刷のレイアウト"))
  })

  it("閉じた見出しは、ほかのファイルに移っても閉じたまま", async () => {
    await openMenu()
    await click(sectionButton("関連ファイル"))
    flush()
    expect(sectionButton("関連ファイル").getAttribute("aria-expanded")).toBe("false")
    await click(columnRow("印刷のレイアウト"))
    expect(heading()).toBe("印刷のレイアウト")
    expect(sectionButton("関連ファイル").getAttribute("aria-expanded")).toBe("false")
    expect(sectionButton("他からのリンク").getAttribute("aria-expanded")).toBe("true")
  })

  it("URL の値を押すと、webview の中で移らずに既定のブラウザで開くよう頼む", async () => {
    const { requests } = await openMenu()
    requests.length = 0
    const pr = "https://github.com/example/recipe-app/pull/128"
    const url = within(column()).getByRole("link", { name: pr })
    const event = new MouseEvent("click", { bubbles: true, cancelable: true })
    url.dispatchEvent(event)
    await tick()
    expect(event.defaultPrevented).toBe(true)
    expect(requests).toEqual([{ cmd: "open_url", args: { url: pr } }])
  })

  it("同じツリーで索引をもらい直しても、右の列の行を作り直さずフォーカスを保つ", async () => {
    const state = { open: linked as Index | null }
    await renderApp(state)
    await openFolders("projects", "recipe-app", "tasks")
    await click(rowOf("献立を印刷できるようにする"))
    const row = columnRow("用紙の大きさを決める")
    row.focus()
    state.open = { ...linked, docs: [...linked.docs, doc("projects/recipe-app/inbox/2026-11-17-note.md", "メモ")] }
    await emit("tree-changed")
    await vi.waitFor(() => expect(within(tree()).queryByText("inbox")).not.toBeNull())
    expect(columnRow("用紙の大きさを決める")).toBe(row)
    expect(document.activeElement).toBe(row)
  })
})

describe("App の島", () => {
  const SAMPLE = "projects/recipe-app/artifacts/assets/print-sample.png"
  const NOTES = "projects/recipe-app/artifacts/assets/notes.txt"
  const PHOTO = "projects/recipe-app/inbox/2026-11-12-photo.png"
  const island: Index = {
    ...linked,
    docs: [
      ...linked.docs.map((d) =>
        d.path === MENU
          ? {
              ...d,
              description: "1 週間分の献立を A4 の 1 枚に収めて印刷する",
              links: [{ ...relative("body", LAYOUT), raw: "../artifacts/print-layout.md" }],
            }
          : d,
      ),
      doc(SAMPLE, "", { is_md: false, asset: true }),
      doc(NOTES, "", { is_md: false, asset: true }),
      doc(PHOTO, "", { is_md: false }),
    ],
  }
  const menuBody = [
    "",
    "# 献立を印刷できるようにする",
    "",
    "## 目的",
    "1 週間分を [印刷のレイアウト](../artifacts/print-layout.md) のとおりに並べる。",
  ].join("\n")
  const body = () => shown().querySelector(".prose")

  async function openMenu(state: Backend) {
    const result = await renderApp(state)
    await openFolders("projects", "recipe-app", "tasks")
    await click(rowOf("献立を印刷できるようにする"))
    await vi.waitFor(() => expect(body()).not.toBeNull())
    return result
  }

  it("見出しの下に description、その下に本文を出す。本文の見出し（# の行）は重ねて出さない", async () => {
    const { requests } = await openMenu({ open: island, bodies: { [MENU]: menuBody } })
    expect(requests).toContainEqual({ cmd: "body", args: { path: MENU } })
    const header = shown().querySelector("header")!
    expect([...header.children].map((el) => `${el.tagName} ${el.textContent}`)).toEqual([
      "H1 献立を印刷できるようにする",
      "P 1 週間分の献立を A4 の 1 枚に収めて印刷する",
    ])
    expect(within(surface()).getAllByRole("heading").map((h) => h.textContent)).toEqual([
      "献立を印刷できるようにする",
      "目的",
    ])
  })

  it("本文のリンクを押すと、webview の中で移らずに新しいタブで開く", async () => {
    await openMenu({ open: island, bodies: { [MENU]: menuBody } })
    const link = within(body() as HTMLElement).getByRole("link", { name: "印刷のレイアウト" })
    const event = new MouseEvent("click", { bubbles: true, cancelable: true })
    link.dispatchEvent(event)
    flush()
    expect(event.defaultPrevented).toBe(true)
    expect(tabNames()).toEqual(["献立を印刷できるようにする", "印刷のレイアウト"])
    await vi.waitFor(() => expect(heading()).toBe("印刷のレイアウト"))
  })

  it("開いているタブに移っても、本文を読み直さず、描いたまま出す", async () => {
    const { requests } = await openMenu({ open: island, bodies: { [MENU]: menuBody } })
    const menu = body()
    within(menu as HTMLElement).getByRole("link", { name: "印刷のレイアウト" }).click()
    flush()
    await vi.waitFor(() => expect(heading()).toBe("印刷のレイアウト"))
    const asked = requests.filter((r) => r.cmd === "body").length
    await click(within(tabList()).getAllByRole("tab")[0])
    expect(heading()).toBe("献立を印刷できるようにする")
    expect(body()).toBe(menu)
    expect(requests.filter((r) => r.cmd === "body").length).toBe(asked)
  })

  it("島のスクロールの位置はタブごとに覚え、切り替えると戻す。新しく開いたタブと開き直したタブは上から", async () => {
    await openMenu({ open: island, bodies: { [MENU]: menuBody } })
    // jsdom はスクロールしないので、位置を持たせて scrollTo を見る
    const scrolled: number[] = []
    let top = 0
    Object.defineProperty(surface(), "scrollTop", { get: () => top, configurable: true })
    surface().scrollTo = ((options: ScrollToOptions) => {
      top = options.top ?? 0
      scrolled.push(top)
    }) as HTMLElement["scrollTo"]
    const frame = () => new Promise((resolve) => requestAnimationFrame(resolve))
    const scrollTo = (to: number) => {
      top = to
      surface().dispatchEvent(new Event("scroll"))
    }

    scrollTo(320)
    within(body() as HTMLElement).getByRole("link", { name: "印刷のレイアウト" }).click()
    flush()
    await frame()
    expect(heading()).toBe("印刷のレイアウト")
    expect(scrolled).toEqual([0])

    scrollTo(80)
    await click(within(tabList()).getAllByRole("tab")[0])
    await frame()
    expect(heading()).toBe("献立を印刷できるようにする")
    expect(scrolled).toEqual([0, 320])

    // 閉じたタブの位置は忘れる
    press("w", { metaKey: true })
    await frame()
    expect(heading()).toBe("印刷のレイアウト")
    expect(scrolled).toEqual([0, 320, 80])
    await click(rowOf("献立を印刷できるようにする"))
    await frame()
    expect(scrolled).toEqual([0, 320, 80, 0])
  })

  it("ツリーが変わったら本文を読み直す。中身が同じなら描き直さない", async () => {
    const state: Backend = { open: island, bodies: { [MENU]: menuBody } }
    const { requests } = await openMenu(state)
    const before = body()
    const bodyRequests = () => requests.filter((r) => r.cmd === "body").length
    const asked = bodyRequests()
    // Rust はツリーが変わるたびに索引を作り直して返す（どのファイルも新しいもの）
    state.open = structuredClone({
      ...island,
      docs: [...island.docs, doc("projects/recipe-app/inbox/2026-11-17-note.md", "メモ")],
    })
    await emit("tree-changed")
    await vi.waitFor(() => expect(within(tree()).queryByText("inbox")).not.toBeNull())
    await tick()
    flush()
    expect(bodyRequests()).toBe(asked + 1)
    expect(body()).toBe(before)

    state.open = structuredClone(state.open)
    state.bodies = { [MENU]: `${menuBody}\n\n## 手順\n- [ ] 並べる` }
    await emit("tree-changed")
    await vi.waitFor(() => expect(within(surface()).queryByRole("heading", { name: "手順" })).not.toBeNull())
    expect(surface().querySelector("input[type=checkbox]")).not.toBeNull()
  })

  describe("本文の file.md#見出し のリンク", () => {
    const withAnchor: Index = {
      ...island,
      docs: island.docs.map((d) =>
        d.path === MENU
          ? { ...d, links: [{ ...relative("body", LAYOUT), raw: "../artifacts/print-layout.md#余白", anchor: "余白" }] }
          : d,
      ),
    }
    const bodies = {
      [MENU]: `${menuBody}\n\n[余白の決め方](../artifacts/print-layout.md#余白)`,
      [LAYOUT]: "# 印刷のレイアウト\n\n本文\n\n## 余白\n",
    }
    const frame = () => new Promise((resolve) => requestAnimationFrame(resolve))
    // 隠したタブは読み上げに出ないので、役割ではなく要素で探す
    const panelOf = (title: string) =>
      [...surface().querySelectorAll("h1")].find((h) => h.textContent === title)!.closest<HTMLElement>("[role=tabpanel]")!

    /** 島の位置を持たせ、scrollTo を覚える。島の上の端は 20、見出し「余白」は島の位置から 600 下にある */
    function watchIsland() {
      // jsdom にはレシピの CSS が無いので、島がスクロールする箱だと分かるようにする
      surface().style.overflowY = "auto"
      const scrolled: number[] = []
      let top = 0
      Object.defineProperty(surface(), "scrollTop", { get: () => top, configurable: true })
      surface().scrollTo = ((options: ScrollToOptions) => {
        top = options.top ?? 0
        scrolled.push(top)
      }) as HTMLElement["scrollTo"]
      vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
        // タブを押す所（click）は left・width も読む
        const y = this === surface() ? 20 : this.id === "user-content-余白" ? 620 - top : 0
        return { top: y, left: 0, width: 0, height: 0 } as DOMRect
      })
      const scrollTo = (to: number) => {
        top = to
        surface().dispatchEvent(new Event("scroll"))
      }
      return { scrolled, scrollTo }
    }
    const follow = () => {
      within(body() as HTMLElement).getByRole("link", { name: "余白の決め方" }).click()
      flush()
    }

    afterEach(() => vi.restoreAllMocks())

    it("新しいタブに開くと、本文を描くまでそのタブを隠し、描いたらすぐ見出しの位置で出す（上から描いたものを見せない）", async () => {
      const state: Backend = { open: withAnchor, bodies }
      await openMenu(state)
      const { scrolled } = watchIsland()
      let release!: () => void
      state.bodyHeld = new Promise((resolve) => (release = resolve))

      follow()
      await tick()
      flush()
      expect(tabNames()).toEqual(["献立を印刷できるようにする", "印刷のレイアウト"])
      expect(panelOf("印刷のレイアウト").style.visibility).toBe("hidden")
      await frame()
      expect(scrolled).toEqual([])

      release()
      await vi.waitFor(() => expect(surface().querySelector('[id="user-content-余白"]')).not.toBeNull())
      // 本文を描いたのと同じ流れの中で（次のフレームを待たずに）送り、出す
      expect(scrolled).toEqual([600])
      expect(panelOf("印刷のレイアウト").style.visibility).toBe("")
      for (let i = 0; i < 3; i++) await frame()
      expect(scrolled).toEqual([600])
    })

    it("もう開いているタブに移るときは、そのタブの前の位置を見せず、見出しの位置で出す", async () => {
      await openMenu({ open: withAnchor, bodies })
      const { scrolled, scrollTo } = watchIsland()
      follow()
      await vi.waitFor(() => expect(surface().querySelector('[id="user-content-余白"]')).not.toBeNull())
      scrollTo(150)
      await click(within(tabList()).getAllByRole("tab")[0])
      await frame()
      expect(heading()).toBe("献立を印刷できるようにする")
      const before = scrolled.length

      follow()
      expect(heading()).toBe("印刷のレイアウト")
      expect(scrolled.slice(before)).toEqual([600])
      for (let i = 0; i < 3; i++) await frame()
      expect(scrolled.slice(before)).toEqual([600])
    })

    it("見出しが無ければ、上から出す", async () => {
      await openMenu({
        open: withAnchor,
        bodies: { ...bodies, [LAYOUT]: "# 印刷のレイアウト\n\n本文\n" },
      })
      const { scrolled, scrollTo } = watchIsland()
      scrollTo(320)
      follow()
      await vi.waitFor(() => expect(within(surface()).queryByText("本文")).not.toBeNull())
      expect(scrolled).toEqual([0])
      expect(panelOf("印刷のレイアウト").style.visibility).toBe("")
    })

    it("送る前にそのタブを閉じたら、送る先は忘れる。開き直すと上から出す", async () => {
      const state: Backend = { open: withAnchor, bodies }
      await openMenu(state)
      const { scrolled } = watchIsland()
      let release!: () => void
      state.bodyHeld = new Promise((resolve) => (release = resolve))
      follow()
      await tick()
      flush()
      press("w", { metaKey: true })
      flush()
      expect(tabNames()).toEqual(["献立を印刷できるようにする"])

      release()
      // 本文のリンクで開いたとき、左の列はその行まで開いている
      await click(rowOf("印刷のレイアウト"))
      await vi.waitFor(() => expect(surface().querySelector('[id="user-content-余白"]')).not.toBeNull())
      await frame()
      expect(heading()).toBe("印刷のレイアウト")
      expect(panelOf("印刷のレイアウト").style.visibility).toBe("")
      // 島の位置を戻す所が上（0）へ送るだけで、「余白」（600）へは送らない
      expect(new Set(scrolled)).toEqual(new Set([0]))
    })
  })

  it("画像のファイルは（assets/ の外でも）島に画像を出し、ほかの .md でないファイルは見出しだけ出す", async () => {
    const { requests } = await renderApp({ open: island })
    await openFolders("projects", "recipe-app", "inbox")
    await click(rowOf("2026-11-12-photo.png"))
    expect(shown().querySelector("img")?.getAttribute("src")).toBe(`desk://localhost/${encodeURIComponent(PHOTO)}`)
    await openFolders("artifacts", "assets")
    await click(rowOf("print-sample.png"))
    expect(heading()).toBe("print-sample.png")
    expect(shown().querySelector("img")?.getAttribute("src")).toBe(`desk://localhost/${encodeURIComponent(SAMPLE)}`)
    // 読み上げで分かるよう、alt はファイル名
    expect(shown().querySelector("img")?.getAttribute("alt")).toBe(SAMPLE.slice(SAMPLE.lastIndexOf("/") + 1))
    await click(rowOf("notes.txt"))
    expect(heading()).toBe("notes.txt")
    expect(shown().querySelector("img, .prose")).toBeNull()
    expect(requests.filter((r) => r.cmd === "body")).toEqual([])
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
