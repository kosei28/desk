import { cleanup, render } from "@solidjs/testing-library"
import type { Element, Root, RootContent } from "hast"
import { flush } from "solid-js"
import { afterEach, describe, expect, it, vi } from "vitest"
import type { Doc } from "../../data/types"
import { highlightLater, markdownToHast } from "./markdown"
import { MarkdownBody } from "./markdown-body"

// このファイルは、Shiki がまだどの言語も色を付けていない所から始まる（試しのファイルごとに読み直す）

const current = { path: "projects/recipe-app/wiki/notes.md", links: [] } as unknown as Doc

/** 木の中のコードの塊（pre）。Shiki は塊を、pre 1 つを持つ root に置き換える */
const blocks = (parent: Root | Element): Element[] =>
  parent.children.flatMap((child) => {
    const node = child as Root | RootContent
    return node.type === "root" ? blocks(node) : node.type === "element" && node.tagName === "pre" ? [node] : []
  })

afterEach(() => cleanup())

describe("コードの色を後から付ける", () => {
  it("初めての言語は、本文を色の無いまま先に出し、描いた後で同じ文字の色付きの塊に差し替える。次からは木にするときに付ける", async () => {
    const first = await markdownToHast("前\n\n```ts\nconst a = 1\n```\n\n後")
    expect(blocks(first).map((pre) => pre.properties.dataHighlight)).toEqual([""])
    const { container } = render(() => (
      <MarkdownBody doc={current} tree={first} byPath={new Map()} onOpen={vi.fn()} onOpenUrl={vi.fn()} />
    ))
    flush()
    // 先に出た塊は色が無く、印の属性も残さない
    const plain = container.querySelector("pre")!
    expect(plain.classList.contains("shiki")).toBe(false)
    expect(plain.hasAttribute(highlightLater)).toBe(false)
    expect(plain.textContent).toBe("const a = 1\n")
    await vi.waitFor(() => expect(container.querySelector("pre")?.classList.contains("shiki")).toBe(true))
    expect(container.querySelector("pre")?.textContent).toBe("const a = 1")
    expect(container.querySelectorAll("pre")).toHaveLength(1)
    expect([...container.querySelectorAll("p")].map((p) => p.textContent)).toEqual(["前", "後"])

    // 色を付けたことのある言語は、木にするときに付ける（印は付けない）
    const again = await markdownToHast("```ts\nlet b = 2\n```")
    expect(blocks(again).map((pre) => [pre.properties.dataHighlight, String(pre.properties.class ?? pre.properties.className).split(" ")[0]])).toEqual([
      [undefined, "shiki"],
    ])
    // 初めての言語が 1 つでもあれば、どの塊も後から付ける
    const mixed = await markdownToHast("```ts\nlet c\n```\n\n```py\nx = 1\n```")
    expect(blocks(mixed).map((pre) => pre.properties.dataHighlight)).toEqual(["", ""])
  })
})
