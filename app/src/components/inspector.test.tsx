import { render, screen } from "@solidjs/testing-library"
import { flush } from "solid-js"
import { afterEach, describe, expect, it, vi } from "vitest"
import { click } from "../test/input"
import {
  forgetInspectorSections,
  Inspector,
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

// 見出しの開け閉めの記憶はモジュールにあるので、テストごとに忘れさせる
afterEach(() => {
  forgetInspectorSections()
})

describe("InspectorSection", () => {
  it("見出しに件数を出さず、開いて始め、押すと畳める", () => {
    render(() => (
      <InspectorSection title="リンク">
        <InspectorItem title="印刷のレイアウト" href="#" />
      </InspectorSection>
    ))
    const trigger = screen.getByRole("button", { name: "リンク" })
    expect(trigger.getAttribute("aria-expanded")).toBe("true")
    click(trigger)
    flush()
    expect(trigger.getAttribute("aria-expanded")).toBe("false")
    click(trigger)
    flush()
    expect(trigger.getAttribute("aria-expanded")).toBe("true")
  })

  it("閉じたまま終わった見出しも、忘れさせると開いて始める", () => {
    const first = render(() => (
      <InspectorSection title="リンク">
        <InspectorItem title="印刷のレイアウト" href="#" />
      </InspectorSection>
    ))
    click(screen.getByRole("button", { name: "リンク" }))
    flush()
    first.unmount()
    forgetInspectorSections()
    flush()

    render(() => (
      <InspectorSection title="リンク">
        <InspectorItem title="印刷のレイアウト" href="#" />
      </InspectorSection>
    ))
    expect(screen.getByRole("button", { name: "リンク" }).getAttribute("aria-expanded")).toBe("true")
  })

  it("開け閉めは見出しの言葉ごとに覚え、ほかのファイルに移っても保つ", () => {
    const first = render(() => (
      <InspectorSection title="他からのリンク">
        <InspectorItem title="献立プランナーの設計" href="#" />
      </InspectorSection>
    ))
    click(screen.getByRole("button", { name: "他からのリンク" }))
    flush()
    first.unmount()

    render(() => (
      <Inspector>
        <InspectorSection title="他からのリンク">
          <InspectorItem title="レシピ API の仕組み" href="#" />
        </InspectorSection>
        <InspectorSection title="リンク">
          <InspectorItem title="印刷のレイアウト" href="#" />
        </InspectorSection>
      </Inspector>
    ))
    const back = screen.getByRole("button", { name: "他からのリンク" })
    expect(back.getAttribute("aria-expanded")).toBe("false")
    expect(screen.getByRole("button", { name: "リンク" }).getAttribute("aria-expanded")).toBe("true")
    click(back)
    flush()
    expect(back.getAttribute("aria-expanded")).toBe("true")
  })
})

describe("InspectorList", () => {
  it("名前の下にファイルの行を全部出す", () => {
    const titles = Array.from({ length: 12 }, (_, i) => `リンク元のファイル ${i + 1}`)
    render(() => (
      <InspectorList label="子タスク" each={titles}>
        {(title: string) => <InspectorItem title={title} href="#" />}
      </InspectorList>
    ))
    expect(screen.getByText("子タスク")).toBeTruthy()
    expect(screen.getAllByRole("link")).toHaveLength(titles.length)
  })

  it("空のリストは「[]」と出す", () => {
    render(() => (
      <InspectorList label="knowledge" each={[] as string[]}>
        {(title: string) => <InspectorItem title={title} href="#" />}
      </InspectorList>
    ))
    expect(screen.getByText("[]")).toBeTruthy()
    expect(screen.queryAllByRole("link")).toHaveLength(0)
  })
})

describe("InspectorItem", () => {
  it("タスクなら頭に status の印、人が要るなら人の印、場所が分からないものは場所を添える", () => {
    render(() => <InspectorItem title="用紙の大きさを決める" status="todo" needsHuman place="archive" href="#" />)
    const row = screen.getByRole("link")
    expect(row.querySelector("[role=img][aria-label=todo]")).not.toBeNull()
    expect(row.querySelector("[role=img][aria-label=人が要る]")).not.toBeNull()
    expect(row.textContent).toBe("用紙の大きさを決めるarchive")
  })

  it("押すと href へ移らずに onOpen を呼び、⌘ を押しながらなら新しいタブで開く", () => {
    const onOpen = vi.fn()
    render(() => <InspectorItem title="印刷のレイアウト" href="projects/recipe-app/artifacts/print-layout.md" onOpen={onOpen} />)
    const row = screen.getByRole("link")
    const plain = new MouseEvent("click", { bubbles: true, cancelable: true })
    const withMeta = new MouseEvent("click", { bubbles: true, cancelable: true, metaKey: true })
    row.dispatchEvent(plain)
    row.dispatchEvent(withMeta)
    expect(onOpen.mock.calls).toEqual([[false], [true]])
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
    render(() => <InspectorUrl href={href} />)
    const link = screen.getByRole("link")
    expect(link.getAttribute("href")).toBe(href)
    expect(link.textContent).toBe(href)
    const breaks = Array.from(link.querySelectorAll("wbr")).map((wbr) => wbr.previousSibling?.textContent)
    expect(breaks).toEqual(["/", "/", "/", "/", "/"])
  })
})
