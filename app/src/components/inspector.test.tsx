import { render, screen } from "@solidjs/testing-library"
import { createSignal, flush } from "solid-js"
import { describe, expect, it, vi } from "vitest"
import { click } from "../test/input"
import {
  InspectorHead,
  InspectorHuman,
  InspectorItem,
  InspectorList,
  InspectorPills,
  InspectorRow,
  InspectorSection,
  InspectorStatus,
  InspectorTable,
  InspectorTag,
  InspectorUrl,
} from "./inspector"

describe("InspectorSection", () => {
  it("見出しに件数を出さず、開け閉めは呼ぶ側の値に従い、押すと onOpenChange を呼ぶ", () => {
    const [open, setOpen] = createSignal(true)
    const onOpenChange = vi.fn((next: boolean) => setOpen(next))
    render(() => (
      <InspectorSection title="リンク" open={open()} onOpenChange={onOpenChange}>
        <InspectorItem title="印刷のレイアウト" href="#" onOpen={() => {}} />
      </InspectorSection>
    ))
    const trigger = screen.getByRole("button", { name: "リンク" })
    expect(trigger.getAttribute("aria-expanded")).toBe("true")
    click(trigger)
    flush()
    expect(onOpenChange.mock.calls.map(([next]) => next)).toEqual([false])
    expect(trigger.getAttribute("aria-expanded")).toBe("false")
    setOpen(true)
    flush()
    expect(trigger.getAttribute("aria-expanded")).toBe("true")
  })
})

describe("InspectorList", () => {
  it("名前の下にファイルの行を全部出す", () => {
    const titles = Array.from({ length: 12 }, (_, i) => `リンク元のファイル ${i + 1}`)
    render(() => (
      <InspectorList label="子タスク" each={titles} key={(title) => title}>
        {(title) => <InspectorItem title={title()} href="#" onOpen={() => {}} />}
      </InspectorList>
    ))
    expect(screen.getByText("子タスク")).toBeTruthy()
    expect(screen.getAllByRole("link")).toHaveLength(titles.length)
  })

  it("空のリストは「[]」と出す", () => {
    render(() => (
      <InspectorList label="knowledge" each={[] as string[]} key={(title) => title}>
        {(title) => <InspectorItem title={title()} href="#" onOpen={() => {}} />}
      </InspectorList>
    ))
    expect(screen.getByText("[]")).toBeTruthy()
    expect(screen.queryAllByRole("link")).toHaveLength(0)
  })
})

describe("InspectorItem", () => {
  it("タスクなら頭に status の印、人が要るなら人の印、場所が分からないものは場所を添える", () => {
    render(() => <InspectorItem title="用紙の大きさを決める" status="todo" needsHuman place="archive" href="#" onOpen={() => {}} />)
    const row = screen.getByRole("link")
    expect(row.querySelector("[role=img][aria-label=todo]")).not.toBeNull()
    expect(row.querySelector("[role=img][aria-label=人が要る]")).not.toBeNull()
    expect(row.textContent).toBe("用紙の大きさを決めるarchive")
  })

  it("押すと href へ移らずに onOpen を呼ぶ。⌘ を押しながらでも同じ", () => {
    const onOpen = vi.fn()
    render(() => <InspectorItem title="印刷のレイアウト" href="projects/recipe-app/artifacts/print-layout.md" onOpen={onOpen} />)
    const row = screen.getByRole("link")
    const plain = new MouseEvent("click", { bubbles: true, cancelable: true })
    const withMeta = new MouseEvent("click", { bubbles: true, cancelable: true, metaKey: true })
    row.dispatchEvent(plain)
    row.dispatchEvent(withMeta)
    expect(onOpen).toHaveBeenCalledTimes(2)
    expect(plain.defaultPrevented).toBe(true)
    expect(withMeta.defaultPrevented).toBe(true)
  })

  it("行はいつも href を持ち、キーボードで届く", () => {
    render(() => <InspectorItem title="印刷のレイアウト" href="projects/recipe-app/artifacts/print-layout.md" onOpen={() => {}} />)
    const row = screen.getByRole("link")
    expect(row.getAttribute("href")).toBe("projects/recipe-app/artifacts/print-layout.md")
    row.focus()
    expect(document.activeElement).toBe(row)
  })
})

describe("InspectorHead", () => {
  it("パスの下に札を並べる。ステータスの札は値だけを読ませ、要対応とタグは文字だけ", () => {
    render(() => (
      <InspectorHead path="projects/recipe-app/tasks/2026-11-10-decide-paper-size.md">
        <InspectorPills>
          <InspectorStatus status="todo" />
          <InspectorHuman />
          <InspectorTag>print</InspectorTag>
        </InspectorPills>
      </InspectorHead>
    ))
    expect(screen.queryAllByRole("img")).toHaveLength(0)
    expect(screen.getByText("todo")).toBeTruthy()
    expect(screen.getByText("要対応")).toBeTruthy()
    expect(screen.getByText("print").textContent).toBe("print")
  })
})

describe("InspectorTable", () => {
  it("項目の名前と値を組にして並べる", () => {
    render(() => (
      <InspectorTable>
        <InspectorRow label="branch">feature/print</InspectorRow>
        <InspectorRow label="reviewer">ichiro</InspectorRow>
      </InspectorTable>
    ))
    const names = Array.from(document.querySelectorAll("dt")).map((dt) => dt.textContent)
    const values = Array.from(document.querySelectorAll("dd")).map((dd) => dd.textContent)
    expect(names).toEqual(["branch", "reviewer"])
    expect(values).toEqual(["feature/print", "ichiro"])
  })
})

describe("InspectorUrl", () => {
  it("/ の後ろにだけ折れる所を置き、ハイフンの前後には置かない", () => {
    const href = "https://example.com/recipe-app/pull/128"
    render(() => <InspectorUrl href={href} onOpen={() => {}} />)
    const link = screen.getByRole("link")
    expect(link.getAttribute("href")).toBe(href)
    expect(link.textContent).toBe(href)
    const breaks = Array.from(link.querySelectorAll("wbr")).map((wbr) => wbr.previousSibling?.textContent)
    expect(breaks).toEqual(["/", "/", "/", "/", "/"])
  })

  it("押すと href へ移らずに onOpen を呼ぶ", () => {
    const onOpen = vi.fn()
    render(() => <InspectorUrl href="https://example.com/a" onOpen={onOpen} />)
    const event = new MouseEvent("click", { bubbles: true, cancelable: true })
    screen.getByRole("link").dispatchEvent(event)
    expect(onOpen).toHaveBeenCalledTimes(1)
    expect(event.defaultPrevented).toBe(true)
  })
})
