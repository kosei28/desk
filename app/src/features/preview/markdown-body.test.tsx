import { cleanup, render } from "@solidjs/testing-library"
import { flush } from "solid-js"
import { afterEach, describe, expect, it, vi } from "vitest"
import type { Doc, Link } from "../../data/types"
import { markdownToHast } from "./markdown"
import { MarkdownBody } from "./markdown-body"

const doc = (path: string, fields: Partial<Doc> = {}): Doc => ({
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
  title: "",
  description: "",
  now: "",
  status: "",
  needs_human: false,
  base: "",
  headings: [],
  links: [],
  ...fields,
})

/** core が本文から読んだリンク（raw は書いてあるとおり、target はルートからのパス、anchor は # の後ろ） */
const bodyLink = (raw: string, target: string, form: Link["form"] = "relative"): Link => ({
  key: "body",
  text: "",
  raw,
  line: 0,
  start: null,
  form,
  target,
  anchor: form === "external" ? "" : (raw.split("#")[1] ?? ""),
})

// 架空のツリー
const here = "projects/recipe-app/tasks/2026-11-09-printable-menu.md"
const docs = [
  doc("projects/recipe-app/artifacts/print-layout.md"),
  doc("projects/recipe-app/artifacts/用紙 の比べ.md"),
  doc("projects/recipe-app/tasks/2026-11-10-decide-paper-size.md"),
  doc("projects/recipe-app/artifacts/assets/print-sample.png", { is_md: false, asset: true }),
  doc("projects/recipe-app/artifacts/assets/print-sample@2x.png", { is_md: false, asset: true }),
  doc("projects/recipe-app/inbox/2026-11-12-photo.png", { is_md: false }),
]

async function show(markdown: string, links: Link[] = []) {
  const tree = await markdownToHast(markdown)
  const current = doc(here, { links })
  const byPath = new Map([current, ...docs].map((d) => [d.path, d]))
  const onOpen = vi.fn<(path: string, anchor?: string) => void>()
  const onOpenUrl = vi.fn<(url: string) => void>()
  const result = render(() => (
    <MarkdownBody doc={current} tree={tree} byPath={byPath} onOpen={onOpen} onOpenUrl={onOpenUrl} />
  ))
  flush()
  return Object.assign(result, { onOpen, onOpenUrl })
}

/** 押す。webview の中で移らない（既定の動きを止めた）かを返す */
function press(element: Element) {
  const event = new MouseEvent("click", { bubbles: true, cancelable: true })
  element.dispatchEvent(event)
  return event.defaultPrevented
}

afterEach(cleanup)

describe("MarkdownBody", () => {
  it("GFM を描き、見出し（最初の # の見出し）は島の見出しに出すので本文に出さない", async () => {
    const { container } = await show(
      [
        "前置き",
        "",
        "# 献立を印刷できるようにする",
        "",
        "## 進め方",
        "",
        "| 用紙 | 枚数 |",
        "| --- | ---: |",
        "| A4 | 1 |",
        "",
        "- [x] 並べる",
        "- [ ] 印刷する",
        "",
        "~~B5~~ と https://example.com/menu",
        "",
        "# 二つ目の h1",
      ].join("\n"),
    )
    const body = container.firstElementChild!
    expect([...body.querySelectorAll("h1, h2")].map((h) => `${h.tagName} ${h.textContent}`)).toEqual([
      "H2 進め方",
      "H1 二つ目の h1",
    ])
    expect(body.querySelector("td:last-child")?.getAttribute("style")).toContain("text-align: right")
    const boxes = [...body.querySelectorAll<HTMLInputElement>("input[type=checkbox]")]
    expect(boxes.map((box) => [box.checked, box.disabled])).toEqual([
      [true, true],
      [false, true],
    ])
    expect(body.querySelector("del")?.textContent).toBe("B5")
    expect(body.querySelector("a")?.getAttribute("href")).toBe("https://example.com/menu")
  })

  it("# の見出しが無ければ、最初の見出しを外す", async () => {
    const { container } = await show("## 進め方\n\n本文\n\n## 次")
    expect([...container.querySelectorAll("h2")].map((h) => h.textContent)).toEqual(["次"])
  })

  it("引用・箇条・脚注の中の見出しは数えない（core の見出しと同じ）", async () => {
    // 引用の中の # は見出しにならず、外の # の見出しを外す
    const quoted = await show("> # 引用の見出し\n\n# 本当の見出し\n\n本文")
    expect([...quoted.container.querySelectorAll("h1")].map((h) => h.textContent)).toEqual(["引用の見出し"])
    expect(quoted.container.querySelector("blockquote h1")?.textContent).toBe("引用の見出し")
    cleanup()
    // 箇条・脚注の中にしか見出しが無ければ（島の見出しはファイル名）、何も外さない
    const listed = await show("- # 箇条の見出し\n\n本文[^1]\n\n[^1]: ## 脚注の見出し")
    expect(listed.container.querySelector("li h1")?.textContent).toBe("箇条の見出し")
    expect(listed.container.querySelector("[data-footnotes] li h2")?.textContent).toBe("脚注の見出し")
  })

  it("生の HTML（details・div）で囲んだ見出しも、Markdown としてはルートの直下なので外す（core の見出しと同じ）", async () => {
    for (const [tag, inside] of [["details", "<summary>開く</summary>\n\n"], ["div", ""]]) {
      const { container } = await show(`<${tag}>\n${inside}# 囲んだ見出し\n\n本文\n\n</${tag}>`)
      expect(container.querySelector(tag)).not.toBeNull()
      expect(container.querySelector("h1")).toBeNull()
      expect(container.textContent).toContain("本文")
      cleanup()
    }
  })

  it("生の HTML は、GitHub とほぼ同じ許可リストで消毒して描く", async () => {
    const { container } = await show(
      [
        '<details open><summary>メモ</summary><kbd>⌘</kbd> と H<sub>2</sub>O</details>',
        "",
        '<p class="x" style="color: red" onclick="alert(1)">飾り</p>',
        "",
        '<script>alert(1)</script><iframe src="https://example.com"></iframe>',
        "",
        '<img src="x.png" onerror="alert(1)"> <a href="javascript:alert(1)">危ない</a> <a id="top">印</a>',
      ].join("\n"),
    )
    const body = container.firstElementChild!
    expect(body.querySelector("details summary")?.textContent).toBe("メモ")
    expect(body.querySelector("kbd")?.textContent).toBe("⌘")
    expect(body.querySelector("sub")?.textContent).toBe("2")
    const p = [...body.querySelectorAll("p")].find((el) => el.textContent === "飾り")!
    expect([p.getAttribute("class"), p.getAttribute("style"), p.getAttribute("onclick")]).toEqual([null, null, null])
    expect(body.querySelector("script, iframe, [onerror]")).toBeNull()
    const links = [...body.querySelectorAll("a")]
    expect(links.find((a) => a.textContent === "危ない")?.hasAttribute("href")).toBe(false)
    expect(links.find((a) => a.textContent === "印")?.id).toBe("user-content-top")
  })

  it("中のファイルを指すリンクは、移らずに onOpen でそのファイルを開く。日本語や空白のパス、生の HTML の a も", async () => {
    const links = [
      bodyLink("../artifacts/print-layout.md#余白", "projects/recipe-app/artifacts/print-layout.md"),
      bodyLink("../artifacts/用紙 の比べ.md", "projects/recipe-app/artifacts/用紙 の比べ.md"),
      bodyLink("2026-11-10-decide-paper-size.md", "projects/recipe-app/tasks/2026-11-10-decide-paper-size.md"),
    ]
    const { getByText, onOpen, onOpenUrl } = await show(
      [
        "[レイアウト](../artifacts/print-layout.md#余白) と [比べ](<../artifacts/用紙 の比べ.md>)",
        "",
        '<a href="2026-11-10-decide-paper-size.md">用紙</a>',
      ].join("\n"),
      links,
    )
    expect(press(getByText("レイアウト"))).toBe(true)
    expect(press(getByText("比べ"))).toBe(true)
    expect(press(getByText("用紙"))).toBe(true)
    // # の後ろ（見出しの名前）は、開いた先の本文の id にして渡す
    expect(onOpen.mock.calls).toEqual([
      ["projects/recipe-app/artifacts/print-layout.md", "user-content-余白"],
      ["projects/recipe-app/artifacts/用紙 の比べ.md", undefined],
      ["projects/recipe-app/tasks/2026-11-10-decide-paper-size.md", undefined],
    ])
    expect(onOpenUrl).not.toHaveBeenCalled()
  })

  it("外の URL（http・https・mailto）は onOpenUrl で開き、切れたリンク・ツリーの外は何もしない", async () => {
    const links = [
      bodyLink("https://github.com/example/recipe-app/pull/128", "https://github.com/example/recipe-app/pull/128", "external"),
      bodyLink("../artifacts/missing.md", "projects/recipe-app/artifacts/missing.md"),
      bodyLink("../../../../outside.md", "../outside.md"),
    ]
    const { getByText, onOpen, onOpenUrl } = await show(
      [
        "[PR](https://github.com/example/recipe-app/pull/128) [メール](mailto:cook@example.com) https://example.com/裸",
        "",
        "[切れた](../artifacts/missing.md) [外](../../../../outside.md)",
      ].join("\n"),
      links,
    )
    for (const text of ["PR", "メール", "https://example.com/裸", "切れた", "外"]) {
      expect(press(getByText(text)), text).toBe(true)
    }
    expect(onOpenUrl.mock.calls).toEqual([
      ["https://github.com/example/recipe-app/pull/128"],
      ["mailto:cook@example.com"],
      ["https://example.com/%E8%A3%B8"],
    ])
    expect(onOpen).not.toHaveBeenCalled()
  })

  it("# のリンク（脚注など）は、いちばん近いスクロールする祖先（島）だけを動かして、本文の中のその場所へ送る", async () => {
    const current = doc(here)
    const byPath = new Map([current, ...docs].map((d) => [d.path, d]))
    const onOpen = vi.fn<(path: string) => void>()
    const onOpenUrl = vi.fn<(url: string) => void>()
    let island!: HTMLDivElement
    const tree = await markdownToHast("本文[^1]\n\n[^1]: 注")
    // 外の箱（overflow: hidden）は動かさない
    const { getByText, container } = render(() => (
      <div style={{ overflow: "hidden" }}>
        <div ref={island} style={{ "overflow-y": "auto", "padding-top": "48px" }}>
          <MarkdownBody doc={current} tree={tree} byPath={byPath} onOpen={onOpen} onOpenUrl={onOpenUrl} />
        </div>
      </div>
    ))
    flush()
    const outer = container.firstElementChild as HTMLElement
    const scrolls = new Map<Element, unknown[]>()
    for (const box of [outer, island]) box.scrollTo = ((options: unknown) => scrolls.set(box, [...(scrolls.get(box) ?? []), options])) as never
    island.scrollTop = 100
    // 島の上の端は 20、脚注は 620、本文の参照は 70 にある
    const ref = getByText("1")
    const rect = vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
      const top = this === island ? 20 : this.tagName === "LI" ? 620 : this === ref ? 70 : 0
      return { top } as DOMRect
    })
    try {
      expect(press(ref)).toBe(true)
      // 戻るリンクは本文の参照へ
      expect(press(container.querySelector("[data-footnote-backref]")!)).toBe(true)
      expect(scrolls.get(island)).toEqual([{ top: 100 + 600 - 48 }, { top: 100 + 50 - 48 }])
      expect(scrolls.has(outer)).toBe(false)
      expect(onOpen).not.toHaveBeenCalled()
      expect(onOpenUrl).not.toHaveBeenCalled()
    } finally {
      rect.mockRestore()
    }
  })

  it("画像は、ローカルのファイルなら（ツリーの中のどこでも、外でも）Rust の scheme から、https ならそのまま読む。picture の source も", async () => {
    const links = [
      bodyLink("../artifacts/assets/print-sample.png", "projects/recipe-app/artifacts/assets/print-sample.png"),
      bodyLink("../artifacts/assets/print-sample@2x.png", "projects/recipe-app/artifacts/assets/print-sample@2x.png"),
      bodyLink("../inbox/2026-11-12-photo.png", "projects/recipe-app/inbox/2026-11-12-photo.png"),
      bodyLink("../../../../photos/外.png", "../photos/外.png"),
      bodyLink("/Users/someone/Pictures/a.png", "/Users/someone/Pictures/a.png", "absolute"),
      bodyLink("~/Pictures/b.png", "~/Pictures/b.png", "home"),
      bodyLink("https://example.com/a.png", "", "external"),
      bodyLink("https://example.com/b.png", "", "external"),
    ]
    const { container } = await show(
      [
        "![見本](../artifacts/assets/print-sample.png) ![assets の外](../inbox/2026-11-12-photo.png) ![ツリーの外](../../../../photos/外.png)",
        "",
        "![絶対](/Users/someone/Pictures/a.png) ![ホーム](~/Pictures/b.png) ![外の URL](https://example.com/a.png) ![読んでいない](other.png)",
        "",
        '<picture><source srcset="../artifacts/assets/print-sample.png 1x, ../artifacts/assets/print-sample@2x.png 2x, https://example.com/b.png 3x"><img src="../artifacts/assets/print-sample.png" alt="並べた見本"></picture>',
      ].join("\n"),
      links,
    )
    const src = (alt: string) => container.querySelector(`img[alt="${alt}"]`)?.getAttribute("src") ?? null
    const local = (path: string) => `desk://localhost/${encodeURIComponent(path)}`
    expect(src("見本")).toBe(local("projects/recipe-app/artifacts/assets/print-sample.png"))
    expect(src("assets の外")).toBe(local("projects/recipe-app/inbox/2026-11-12-photo.png"))
    expect(src("ツリーの外")).toBe(local("../photos/外.png"))
    expect(src("絶対")).toBe(local("/Users/someone/Pictures/a.png"))
    expect(src("ホーム")).toBe(local("~/Pictures/b.png"))
    expect(src("外の URL")).toBe("https://example.com/a.png")
    // core の読んだリンクに無いもの（ここでは links に入れていない）は読まない
    expect(src("読んでいない")).toBeNull()
    expect(src("並べた見本")).toBe(local("projects/recipe-app/artifacts/assets/print-sample.png"))
    expect(container.querySelector("source")?.getAttribute("srcset")).toBe(
      `${local("projects/recipe-app/artifacts/assets/print-sample.png")} 1x, ${local("projects/recipe-app/artifacts/assets/print-sample@2x.png")} 2x, https://example.com/b.png 3x`,
    )
  })

  it("見出しに GitHub と同じ id を付ける。島の見出しに出す見出しも数えるので、同じ文字の見出しの番号も GitHub と揃う", async () => {
    const { container } = await show(
      [
        "# 概要",
        "",
        "## 3. お気に入りの解除で、バリエーションが無いとき",
        "",
        "## 概要",
        "",
        "> ### Note: `code` と **太字**",
        "",
        '<h2 id="手で付けた">生の HTML</h2>',
      ].join("\n"),
    )
    expect([...container.querySelectorAll("h2, h3")].map((h) => h.id)).toEqual([
      "user-content-3-お気に入りの解除でバリエーションが無いとき",
      "user-content-概要-1",
      "user-content-note-code-と-太字",
      "user-content-手で付けた",
    ])
  })
})

describe("MarkdownBody の見出しへのリンク", () => {
  /** 島（縦にスクロールする箱）の中に描く。島の上の端は 20、見出し「余白」は 520 にある */
  async function showInIsland(markdown: string) {
    const tree = await markdownToHast(markdown)
    const current = doc(here)
    const byPath = new Map([current, ...docs].map((d) => [d.path, d]))
    let island!: HTMLDivElement
    const result = render(() => (
      <div ref={island} style={{ "overflow-y": "auto", "padding-top": "48px" }}>
        <MarkdownBody
          doc={current}
          tree={tree}
          byPath={byPath}
          onOpen={() => {}}
          onOpenUrl={() => {}}
        />
      </div>
    ))
    flush()
    const scrolls: unknown[] = []
    island.scrollTo = ((options: unknown) => scrolls.push(options)) as never
    island.scrollTop = 100
    vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
      return { top: this === island ? 20 : this.id === "user-content-余白" ? 520 : 0 } as DOMRect
    })
    return Object.assign(result, { scrolls })
  }

  afterEach(() => vi.restoreAllMocks())

  it("# 見出しのリンクは、本文の中のその見出しへ島を送る", async () => {
    const { getByText, scrolls } = await showInIsland("# 題\n\n[余白へ](#余白)\n\n## 余白")
    expect(press(getByText("余白へ"))).toBe(true)
    expect(scrolls).toEqual([{ top: 100 + 500 - 48 }])
  })
})
