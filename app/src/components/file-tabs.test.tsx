import { render, screen } from "@solidjs/testing-library"
import { Tabs } from "@ui-sdk/solid/tabs"
import { createSignal, flush } from "solid-js"
import { describe, expect, it, vi } from "vitest"
import { click } from "../test/input"
import { FileTabList, type TabEntry } from "./file-tabs"

const entries: TabEntry[] = [
  { path: "tasks/2026-11-09-printable-menu.md", title: "献立を印刷できるようにする", status: "inprogress" },
  { path: "artifacts/print-layout.md", title: "印刷のレイアウト" },
]

function renderTabs() {
  const [value, setValue] = createSignal(entries[0].path)
  const onClose = vi.fn()
  render(() => (
    <Tabs value={value()} onValueChange={(next) => setValue(next as string)}>
      <FileTabList tabs={entries} onClose={onClose} />
    </Tabs>
  ))
  return { value, onClose }
}

describe("FileTabList", () => {
  it("開いているファイルを、見出しを名前にしたタブで並べる", () => {
    renderTabs()
    const list = screen.getByRole("tablist", { name: "開いているファイル" })
    const tabs = screen.getAllByRole("tab")
    expect(list.contains(tabs[0])).toBe(true)
    expect(tabs.map((tab) => tab.textContent)).toEqual(["献立を印刷できるようにする", "印刷のレイアウト"])
  })

  it("同じパスのタブは、渡し直しても作り直さず、見出しだけ変える", () => {
    const [tabs, setTabs] = createSignal(entries)
    render(() => (
      <Tabs value={entries[0].path}>
        <FileTabList tabs={tabs()} onClose={() => {}} />
      </Tabs>
    ))
    const before = screen.getByRole("tab", { name: "印刷のレイアウト" })
    setTabs([entries[0], { ...entries[1], title: "印刷の版面" }])
    flush()
    const after = screen.getByRole("tab", { name: "印刷の版面" })
    expect(after).toBe(before)
  })

  it("タブを押すと、そのパスに切り替わる", () => {
    const { value } = renderTabs()
    click(screen.getByRole("tab", { name: "印刷のレイアウト" }))
    flush()
    expect(value()).toBe("artifacts/print-layout.md")
  })

  it("閉じるボタンはタブ順に入れず、押すとそのタブのパスで onClose を呼ぶ", () => {
    const { onClose } = renderTabs()
    const close = screen.getByRole("button", { name: "印刷のレイアウト を閉じる" })
    expect(close.tabIndex).toBe(-1)
    close.click()
    expect(onClose).toHaveBeenCalledWith("artifacts/print-layout.md")
  })

  it("タブを包む要素にパスを持たせ、右クリックのメニューがどのタブかを読めるようにする", () => {
    renderTabs()
    const tab = screen.getByRole("tab", { name: "印刷のレイアウト" })
    expect(tab.closest<HTMLElement>("[data-path]")?.dataset.path).toBe("artifacts/print-layout.md")
  })
})
