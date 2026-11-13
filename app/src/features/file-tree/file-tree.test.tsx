import { render, screen } from "@solidjs/testing-library"
import { createSignal, flush } from "solid-js"
import { describe, expect, it, vi } from "vitest"
import { tree } from "../../../styled-system/recipes"
import { click, keys } from "../../test/input"
import { FileTree, type FileEntry } from "./file-tree"

const files: FileEntry[] = [
  { path: "projects/recipe-app/tasks/2026-11-09-printable-menu.md", title: "献立を印刷できるようにする" },
  { path: "README.md", title: "作業机" },
  { path: "notes.md" },
  { path: "2026-11-10-decide-paper-size.md", title: "用紙の大きさを決める", status: "todo", needsHuman: true },
]

function renderTree(initial: string | null = null) {
  const [selected, setSelected] = createSignal(initial)
  const onSelect = vi.fn(setSelected)
  render(() => <FileTree files={files} selected={selected()} onSelect={onSelect} />)
  return Object.assign(setSelected, { onSelect })
}

const labelOf = (text: string) => screen.getByText(text).closest("[data-slot=tree-label]")!
const itemOf = (text: string) => screen.getByText(text).closest<HTMLElement>("[data-slot=tree-item]")!
const selectedRows = () => [...document.querySelectorAll("[data-slot=tree-label][data-selected]")]
const press = (text: string) => click(labelOf(text) as HTMLElement)
const expanded = (text: string) => itemOf(text).getAttribute("aria-expanded")

describe("FileTree", () => {
  it("UI SDK の Tree のパーツに Panda のスロットレシピのクラスを当てる", () => {
    renderTree()
    const classes = tree()
    const root = screen.getByRole("tree", { name: "ファイル" })
    expect(root.classList.contains(classes.root)).toBe(true)

    expect(labelOf("作業机").classList.contains(classes.label)).toBe(true)
  })

  it("フォルダーを先に並べ、閉じたフォルダーの中は出さない", () => {
    renderTree()
    const items = screen.getAllByRole("treeitem")
    expect(items.map((item) => item.getAttribute("aria-label") ?? item.textContent)).toEqual([
      "projects",
      "作業机",
      "notes.md",
      "用紙の大きさを決める",
    ])
  })

  it("ファイルは見出しで出し、見出しが無ければファイル名で出す", () => {
    renderTree()
    expect(screen.queryByText("README.md")).toBeNull()
    expect(screen.getByText("作業机")).toBeTruthy()
    expect(screen.getByText("notes.md")).toBeTruthy()
  })

  it("タスクの行に status の印と、人が要る印を出す", () => {
    renderTree()
    const label = labelOf("用紙の大きさを決める")
    expect(label.querySelector("[role=img][aria-label=todo]")).not.toBeNull()
    expect(label.querySelector("[role=img][aria-label=人が要る]")).not.toBeNull()
    expect(labelOf("作業机").querySelector("[role=img]")).toBeNull()
  })

  it("印の無いファイルの行は頭の枠を持たず、文字から始める", () => {
    renderTree()
    expect([...labelOf("作業机").children].map((child) => child.textContent)).toEqual(["作業机"])
    expect(labelOf("用紙の大きさを決める").children).toHaveLength(3)
    expect(labelOf("projects").children).toHaveLength(2)
  })

  it("選んだパスの行に選択の状態を付け、選び直すと移す", () => {
    const setSelected = renderTree("README.md")
    expect(labelOf("作業机").hasAttribute("data-selected")).toBe(true)

    setSelected("notes.md")
    flush()
    expect(labelOf("作業机").hasAttribute("data-selected")).toBe(false)
    expect(labelOf("notes.md").hasAttribute("data-selected")).toBe(true)
  })

  it("フォルダーのパスを渡されても、フォルダーの行は選択の状態にしない", () => {
    renderTree("projects")
    expect(selectedRows()).toEqual([])
  })

  it("ファイルの行を押すと選び、onSelect にパスを 1 回だけ渡す", async () => {
    const { onSelect } = renderTree("README.md")
    await press("notes.md")
    expect(onSelect.mock.calls).toEqual([["notes.md"]])
    expect(selectedRows()).toEqual([labelOf("notes.md")])
  })

  it("選んでいるファイルの行を押し直しても、選択を外さない", async () => {
    const { onSelect } = renderTree("README.md")
    await press("作業机")
    expect(selectedRows()).toEqual([labelOf("作業机")])
    expect(onSelect).not.toHaveBeenCalledWith(null)
  })

  it("フォルダーの行を押すと開閉だけして、選択は前のファイルのまま残す", async () => {
    const { onSelect } = renderTree("README.md")

    await press("projects")
    expect(expanded("projects")).toBe("true")
    expect(screen.getByText("recipe-app")).toBeTruthy()
    expect(selectedRows()).toEqual([labelOf("作業机")])

    await press("projects")
    expect(expanded("projects")).toBe("false")
    expect(screen.queryByText("recipe-app")).toBeNull()
    expect(selectedRows()).toEqual([labelOf("作業机")])
    expect(onSelect).not.toHaveBeenCalled()
  })

  it("開いたフォルダーの中のファイルを押すと選ぶ", async () => {
    renderTree()
    await press("projects")
    await press("recipe-app")
    await press("tasks")
    await press("献立を印刷できるようにする")
    expect(selectedRows()).toEqual([labelOf("献立を印刷できるようにする")])
  })

  it("上下の矢印で、見えている行を順に動く。フォルダーにも止まる", async () => {
    renderTree("README.md")
    await press("作業机")
    await keys("ArrowUp")
    expect(document.activeElement).toBe(itemOf("projects"))
    await keys("ArrowDown", "ArrowDown")
    expect(document.activeElement).toBe(itemOf("notes.md"))
    expect(selectedRows()).toEqual([labelOf("作業机")])
  })

  it("Home と End で最初と最後の行へ動く", async () => {
    renderTree("README.md")
    await press("作業机")
    await keys("End")
    expect(document.activeElement).toBe(itemOf("用紙の大きさを決める"))
    await keys("Home")
    expect(document.activeElement).toBe(itemOf("projects"))
  })

  it("右の矢印でフォルダーを開いて中へ入り、左の矢印で親へ戻って閉じる", async () => {
    renderTree("README.md")
    await press("作業机")
    await keys("Home", "ArrowRight")
    expect(expanded("projects")).toBe("true")
    await keys("ArrowRight")
    expect(document.activeElement).toBe(itemOf("recipe-app"))

    await keys("ArrowLeft")
    expect(document.activeElement).toBe(itemOf("projects"))
    expect(expanded("projects")).toBe("true")
    await keys("ArrowLeft")
    expect(expanded("projects")).toBe("false")
    expect(selectedRows()).toEqual([labelOf("作業机")])
  })

  it("Enter はフォルダーなら開閉し、ファイルなら選ぶ", async () => {
    renderTree("README.md")
    await press("作業机")
    await keys("ArrowUp", "Enter")
    expect(expanded("projects")).toBe("true")
    expect(selectedRows()).toEqual([labelOf("作業机")])
    await keys("Enter")
    expect(expanded("projects")).toBe("false")

    await keys("End", "Enter")
    expect(selectedRows()).toEqual([labelOf("用紙の大きさを決める")])
  })

  it("Space はファイルなら選び、選んでいるファイルとフォルダーでは何もしない", async () => {
    const { onSelect } = renderTree("README.md")
    await press("作業机")
    await keys("Space")
    expect(selectedRows()).toEqual([labelOf("作業机")])

    await keys("ArrowUp", "Space")
    expect(expanded("projects")).toBe("false")
    expect(selectedRows()).toEqual([labelOf("作業机")])

    await keys("ArrowDown", "ArrowDown", "Space")
    expect(selectedRows()).toEqual([labelOf("notes.md")])
    expect(onSelect.mock.calls).toEqual([["notes.md"]])
  })

  it("文字を打つと、その文字で始まる見出しの行へ動く", async () => {
    renderTree("README.md")
    await press("作業机")
    await keys("n")
    expect(document.activeElement).toBe(itemOf("notes.md"))
  })

  it("Tab で入る行は 1 つだけで、選んでいるファイルの行", () => {
    renderTree("notes.md")
    const tabbable = screen.getAllByRole("treeitem").filter((item) => item.tabIndex === 0)
    expect(tabbable).toEqual([itemOf("notes.md")])
  })
})
