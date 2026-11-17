import { cleanup, render } from "@solidjs/testing-library"
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks"
import { flush } from "solid-js"
import { afterEach, describe, expect, it, vi } from "vitest"
import type { Doc } from "../../data/types"
import { markdownToHast } from "./markdown"
import { MarkdownBody } from "./markdown-body"
import { drawDiagrams } from "./mermaid-diagram"
// rich.ts が CSS を読む katex
import katex from "katex"

// 架空のツリーのファイル。数式・コード・図はリンクを持たない（図の中のリンクは core の索引に無い）
const current = { path: "projects/recipe-app/wiki/notes.md", links: [] } as unknown as Doc
const menu = "projects/recipe-app/tasks/2026-11-09-printable-menu.md"
const byPath = new Map([[menu, { path: menu } as Doc]])

async function show(markdown: string) {
  const tree = await markdownToHast(markdown)
  const onOpenUrl = vi.fn<(url: string) => void>()
  const onOpen = vi.fn<(path: string, anchor?: string) => void>()
  const result = render(() => (
    <MarkdownBody doc={current} tree={tree} byPath={byPath} onOpen={onOpen} onOpenUrl={onOpenUrl} />
  ))
  flush()
  return Object.assign(result, { onOpenUrl, onOpen })
}

/** KaTeX の数式の数（ブロックの数式も 1 つ） */
const mathCount = (container: Element) => container.querySelectorAll(".katex").length

afterEach(() => {
  cleanup()
  clearMocks()
})

describe("数式", () => {
  // 作り物の文を GitHub（POST /markdown）に投げて数えた、インラインの数式の数（desk の「Markdown のレンダリングの技術選定」）
  const github: [string, number][] = [
    ["Price $5 and $10.", 0],
    ["$20,000 and $30,000", 0],
    ["It costs $5 today.", 0],
    ["echo $HOME and $USER", 0],
    ["zsh の $VAR:e と $B:x は別物", 0],
    ["価格は$5から$10まで", 0],
    ["use ${B} and $B", 0],
    // GitHub 自身の誤り（開きの前も閉じの後も文字ではない）。同じにする
    ["$(pwd) and $(date)", 1],
    ["regex ^a$ and ^b$ both", 0],
    ["PHP: $a = $b;", 0],
    ["between $ and $ signs", 0],
    ["cost: $5-$10", 0],
    ["USD$5 or US$10", 0],
    ["the vars $A, $B and $C", 0],
    ["変数 $FOO と $BAR を使う", 0],
    ["--lease=$B:abc は $B を展開", 0],
    ["Inline $E=mc^2$ here.", 1],
    ["$x$", 1],
    // 和文や英字に直に付けたものは、GitHub は数式にしない
    ["値は$x$です。", 0],
    ["値は $x$ です。", 1],
    ["root $\\sqrt{3}$ ok", 1],
    ["$a+b$, $c+d$.", 2],
    ["$x_1$ and $x_2$", 2],
    ["the $n$th term", 0],
    ["($x$)", 1],
    ["GitHub style $`a+b`$ here", 1],
    ["double $$x+y$$ inline", 1],
    ["**$x$**", 1],
    ["set $\\{a\\}$ here", 1],
    ["$1+1=2$", 1],
    ["面積は$\\pi r^2$。", 0],
    ["in $\\mathbb{R}$.", 1],
    ["the $x$-axis", 1],
    ["$f(x) = x^2$, so", 1],
  ]

  it.each(github)("インラインの数式は GitHub と同じものだけ: %s", async (text, count) => {
    const { container } = await show(text)
    expect(mathCount(container)).toBe(count)
    // 数式にしなかった $ は、書いたとおりの文字で残る
    if (count === 0) expect(container.textContent).toBe(text.replaceAll("**", ""))
  })

  it("見出しの中の zsh の $ は数式にしない（和文と -- が数式の文字にならない）", async () => {
    const { container } = await show(
      "# 題\n\n## zsh では $VAR:e… などの直後のコロンが修飾子になる。--force-with-lease=$B:x は壊れる",
    )
    expect(mathCount(container)).toBe(0)
    expect(container.querySelector("h2")?.textContent).toBe(
      "zsh では $VAR:e… などの直後のコロンが修飾子になる。--force-with-lease=$B:x は壊れる",
    )
  })

  it("$`…`$ は ` を外して数式にする。$$…$$ のブロックと ```math は、行を分けた数式にする", async () => {
    const { container } = await show("式 $`a+b`$ を\n\n$$\n\\sum_i i\n$$\n\n```math\nE = mc^2\n```")
    const inline = container.querySelector("p .katex annotation")
    expect(inline?.textContent).toBe("a+b")
    const blocks = [...container.querySelectorAll(".katex-display annotation")].map((a) => a.textContent?.trim())
    expect(blocks).toEqual(["\\sum_i i", "E = mc^2"])
    // 元の code・pre は残らない
    expect(container.querySelector("code, pre")).toBeNull()
  })

  it("数式の CSS は、rehype-katex が使う KaTeX と同じ版のもの（版が違うと class の名前がずれて組めない）", async () => {
    const { container } = await show("$x^2$")
    // 本文の要素（p）から下の class を、出てくる順に並べる
    const classes = (root: Element) => [...root.querySelectorAll("[class]")].map((element) => element.className)
    const expected = document.createElement("div")
    expected.innerHTML = katex.renderToString("x^2")
    expect(classes(container.querySelector("p")!)).toEqual(classes(expected))
  })

  it("書き損じた数式は、その場に文字で出す", async () => {
    const { container } = await show("式 $\\frac{1}{$ と")
    expect(container.querySelector(".katex-error")?.textContent).toBe("\\frac{1}{")
  })

  it("消毒の後に数式にするので、生の HTML で数式の class を付けても、ほかの class・style は残らない", async () => {
    const { container } = await show('<code class="language-math evil" style="color:red">x</code>')
    expect(mathCount(container)).toBe(1)
    expect(container.querySelector(".evil, [style*='red']")).toBeNull()
  })

  // KaTeX の既定（trust: false）で、リンク・画像・style を出す命令は、その場に文字で出て、属性にならない
  it.each([
    ["\\href{javascript:alert(1)}{x}", "\\href"],
    ["\\url{javascript:alert(1)}", "\\url"],
    ["\\htmlStyle{color:red}{x}", "\\htmlStyle"],
    ["\\includegraphics{x.png}", "\\includegraphics"],
  ])("数式の %s は、リンク・画像・style にしない", async (math, command) => {
    const { container } = await show(`式 $${math}$ と`)
    expect(container.querySelector("p")?.textContent).toContain(command)
    expect(container.querySelector("a, img, [href], [src]")).toBeNull()
    expect(container.querySelector("[style*='color:red'], [style*='color: red']")).toBeNull()
  })
})

describe("コードの色", () => {
  it("言語を書いたコードは、ライトとダークの色を CSS 変数で持つ（色は prose のレシピが選ぶ）", async () => {
    const { container } = await show("```ts\nconst a: number = 1\n```")
    const pre = container.querySelector("pre")
    expect(pre?.classList.contains("shiki")).toBe(true)
    expect(pre?.textContent).toBe("const a: number = 1")
    const keyword = [...container.querySelectorAll<HTMLElement>("pre span span")].find((s) => s.textContent === "const")
    expect(keyword?.style.getPropertyValue("--shiki-light")).toMatch(/^#/)
    expect(keyword?.style.getPropertyValue("--shiki-dark")).toMatch(/^#/)
    // 色そのものは付けない（明暗が替わっても描き直さない）
    expect(keyword?.style.color).toBe("")
  })

  it("言語の別名（sh・js）も読む", async () => {
    const { container } = await show("```sh\necho hi\n```\n\n```js\nlet x\n```")
    expect([...container.querySelectorAll("pre")].map((pre) => pre.classList.contains("shiki"))).toEqual([true, true])
  })

  it("言語の無いコード、知らない言語、本文の中のコードには色を付けない", async () => {
    const { container } = await show("```\nplain\n```\n\n```nosuchlang\nx\n```\n\n本文の `code`")
    expect(container.querySelector(".shiki")).toBeNull()
    expect([...container.querySelectorAll("pre")].map((pre) => pre.textContent)).toEqual(["plain\n", "x\n"])
    expect(container.querySelector("p code")?.textContent).toBe("code")
  })
})

describe("コードの写すボタン", () => {
  it("コードの塊の右上のボタンで、中の文字を写す（末尾の改行は外す）。写したら印を替える", async () => {
    const writeText = vi.fn(async (_text: string) => {})
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true })
    const { container, getAllByRole } = await show("```ts\nconst a = 1\n```\n\n```\nplain\n```\n\n本文の `code`")
    const buttons = getAllByRole("button", { name: "コードを写す" })
    expect(buttons).toHaveLength(2)
    buttons[0]!.click()
    await vi.waitFor(() => expect(writeText).toHaveBeenCalledWith("const a = 1"))
    buttons[1]!.click()
    await vi.waitFor(() => expect(writeText).toHaveBeenLastCalledWith("plain"))
    await vi.waitFor(() => expect(container.querySelectorAll("[aria-label='写した']")).toHaveLength(2))
  })
})

describe("mermaid の図", () => {
  it("```mermaid は pele で SVG の図にし、コードとしては出さない", async () => {
    const { container } = await show("```mermaid\nflowchart LR\n  A[献立] --> B[印刷]\n```")
    const svg = container.querySelector("svg.pele")
    expect(svg).not.toBeNull()
    expect([...svg!.querySelectorAll(".pele-label")].map((label) => label.textContent)).toEqual(["献立", "印刷"])
    expect(container.querySelector("pre, code")).toBeNull()
  })

  // 島に置かれる前に描く順と、隠れていた図を今の幅で組み直すのは、jsdom では作れない（render が onSettled まで済ませ、幅が無い）。
  // ヘッドレスの Chromium で確かめる
  it("drawDiagrams は、幅が変わっていない図を描き直さない", async () => {
    const { container } = await show("```mermaid\nflowchart LR\n  A --> B\n```")
    const svg = container.querySelector("svg.pele")
    drawDiagrams(container)
    flush()
    expect(container.querySelectorAll("svg.pele")).toHaveLength(1)
    expect(container.querySelector("svg.pele")).toBe(svg)
  })

  it("書き損じた図は、その場にエラーを文字で出す", async () => {
    const { container } = await show("```mermaid\nflowchart LR\n  A -->\n```")
    expect(container.querySelector("svg")).toBeNull()
    expect(container.querySelector(".pele-error")?.textContent).toContain("Parse error on line 2")
  })

  it("図の中のリンクは移らず、外の URL は onOpenUrl、ツリーの中のファイルは core に解いてもらって onOpen で開く", async () => {
    // core の resolve_link の代わり。書いた先だけを、core が返す形で返す
    mockIPC((cmd, args) => {
      const { from, raw } = args as { from: string; raw: string }
      if (cmd === "resolve_link" && from === current.path && raw === "../tasks/2026-11-09-printable-menu.md")
        return { key: "body", text: "", raw, line: 0, start: null, form: "relative", target: menu, anchor: "" }
      throw new Error(`unexpected ${cmd} ${raw}`)
    })
    const { container, onOpenUrl, onOpen } = await show(
      '```mermaid\nflowchart LR\n  A[外] --> B[中]\n  click A "https://example.com/menu"\n  click B "../tasks/2026-11-09-printable-menu.md"\n```',
    )
    const press = (element: Element) => {
      const event = new MouseEvent("click", { bubbles: true, cancelable: true })
      element.dispatchEvent(event)
      return event.defaultPrevented
    }
    const [outer, inner] = [...container.querySelectorAll("svg a")]
    expect(press(outer!.querySelector("text")!)).toBe(true)
    await vi.waitFor(() => expect(onOpenUrl).toHaveBeenCalledWith("https://example.com/menu"))
    expect(press(inner!.querySelector("text")!)).toBe(true)
    await vi.waitFor(() => expect(onOpen).toHaveBeenCalledWith(menu, undefined))
    expect(onOpenUrl).toHaveBeenCalledTimes(1)
  })

  it("図の中の javascript: の行き先は、pele が消す", async () => {
    const { container, onOpenUrl, onOpen } = await show(
      '```mermaid\nflowchart LR\n  A[押す] --> B[先]\n  click A "javascript:alert(1)"\n```',
    )
    expect(container.querySelector("svg.pele")).not.toBeNull()
    const targets = [...container.querySelectorAll("svg a")].map(
      (link) => link.getAttribute("href") ?? link.getAttribute("xlink:href") ?? "",
    )
    expect(targets.filter((href) => /javascript:/i.test(href))).toEqual([])
    for (const label of container.querySelectorAll("svg text")) label.dispatchEvent(new MouseEvent("click", { bubbles: true }))
    // 図の中のリンクは非同期に開くので、1 回待ってから見る
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(onOpenUrl).not.toHaveBeenCalled()
    expect(onOpen).not.toHaveBeenCalled()
  })
})
