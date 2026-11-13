import { render, screen } from "@solidjs/testing-library"
import { createAgent } from "@ui-sdk/solid/agent"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { click } from "./test/input"

const invoke = vi.hoisted(() => vi.fn(() => Promise.resolve()))
vi.mock("@tauri-apps/api/core", () => ({ invoke }))

import { App } from "./app"

// 1 つのファイルに 1 つ。外した部品も offPage として覚えているので、作り直さない
const agent = createAgent()

const sheet = () => screen.getByRole("article")
const rowOf = (text: string) => screen.getByText(text).closest<HTMLElement>("[data-slot=tree-label]")!
const mouse = (target: Element, type: "mousedown" | "mouseup", detail: number) =>
  target.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, detail, button: 0 }))

describe("App", () => {
  beforeEach(() => invoke.mockClear())

  it("はじめは紙に、ファイルを選ぶよう出す", () => {
    render(() => <App />)
    expect(sheet().textContent).toBe("ファイルを選ぶと、ここに開く")
  })

  it("ツリーでファイルを押すと、紙にそのパスと見出しを出す", async () => {
    render(() => <App />)
    await click(rowOf("作業机"))
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("作業机")
    expect(sheet().textContent).toContain("README.md")
  })

  it("フォルダーを押しても、紙は前のファイルのまま", async () => {
    render(() => <App />)
    await click(rowOf("作業机"))
    await click(rowOf("projects"))
    expect(screen.getByText("recipe-app")).toBeTruthy()
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("作業机")
  })

  // 帯の高さの中で紙に覆われていない所がどこも帯に当たるかは、レイアウトの話で jsdom では見られない。ここでは、
  // 窓を動かすのが帯の層だけで、紙・ツリーの行・紙の列の入れ物（帯より下の右と下の余白）では動かないことを見る
  it("帯の層を押すと窓を動かし、ダブルクリックでタイトルバーのアクションを送る。紙・ツリーの行・帯より下の余白では何もしない", async () => {
    render(() => <App />)
    const band = screen.getByRole("navigation", { name: "ツリー" }).previousElementSibling!
    const margin = sheet().parentElement!
    const doubleClick = (target: Element) => {
      mouse(target, "mousedown", 1)
      mouse(target, "mouseup", 1)
      mouse(target, "mousedown", 2)
      mouse(target, "mouseup", 2)
    }

    const row = rowOf("作業机")
    await click(row)
    for (const target of [sheet(), row, margin]) doubleClick(target)
    expect(invoke).not.toHaveBeenCalled()

    expect(band.parentElement).toBe(margin.parentElement)
    doubleClick(band)
    expect(invoke.mock.calls).toEqual([["plugin:window|start_dragging"], ["titlebar_double_click"]])
  })

  it("エージェントが、名前でツリーを見つけ、ファイルを選べる。フォルダーは選べない", async () => {
    render(() => <App />)
    const components = agent.snapshot().filter((entry) => !entry.offPage && entry.component !== "more")
    for (const entry of components) {
      expect(entry.label, `${entry.component} ${entry.ref} has no name`).toBeTruthy()
    }
    const fileTree = components.find((entry) => entry.label === "ファイル")
    expect(fileTree?.component).toBe("tree")
    // 値は自分のデータからではなく、エージェントが見るもの（snapshot）から取る（UI SDK の skill の agent のテスト）
    const items = (fileTree as { items?: { value: string; label: string }[] }).items ?? []
    const file = items.find((item) => item.label === "作業机")
    const folder = items.find((item) => item.label === "projects")
    expect(file?.value).toBe("README.md")
    expect(folder).toBeDefined()

    const opened = await agent.act({ steps: [{ ref: "ファイル", action: "setValue", args: [file!.value] }] })
    expect(opened.ok, opened.error?.message).toBe(true)
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("作業机")

    await agent.act({ steps: [{ ref: "ファイル", action: "setValue", args: [folder!.value] }] })
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("作業机")

    const expanded = await agent.act({ steps: [{ ref: "ファイル", action: "expand", args: [folder!.value] }] })
    expect(expanded.ok, expanded.error?.message).toBe(true)
    expect(screen.getByText("recipe-app")).toBeTruthy()
  })
})
