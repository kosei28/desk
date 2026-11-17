// Markdown の本文を hast（HTML の木）にする。描き方の決まりは desk の「Markdown のレンダリングの技術選定」。
// GFM と生の HTML を要素にし、GitHub とほぼ同じ許可リスト（rehype-sanitize の既定）で消毒する。
// 消毒の後に、数式（KaTeX）・コードの色（Shiki）・mermaid の図（pele）を足す。どれもライブラリの出したものなので、消毒にかけない。

import GithubSlugger from "github-slugger"
import type { Element, ElementContent, Root, RootContent } from "hast"
import { toString as hastToString } from "hast-util-to-string"
import type { Heading, Literal, Root as MdastRoot, Text } from "mdast"
import { detectType, load as loadDiagram } from "pele/lazy"
import rehypeRaw from "rehype-raw"
import rehypeSanitize, { defaultSchema } from "rehype-sanitize"
import remarkGfm from "remark-gfm"
import remarkMath from "remark-math"
import remarkParse from "remark-parse"
import remarkRehype from "remark-rehype"
import { unified } from "unified"
import { urlSchemes } from "../../data/url-schemes"

/** 島の見出しに出すので本文から外す見出しの、元の文字の中の位置（hast の position.start.offset と同じ数え方） */
type TitleData = { titleOffset?: number }

/** unified の VFile のうち、ここで使う所（vfile は直に依存しない） */
type File = { data: object; value: unknown }

/**
 * 見出し（core の Doc の title と同じく、最初の # の見出し。無ければ最初の見出し）の位置を覚える。外すのは id を付けた後（rehypeHeadingIds）。
 * core と同じく、引用・箇条・脚注の中の見出しは数えない（ルートの直下の見出しだけから選ぶ）
 */
function remarkFindTitle() {
  return (tree: MdastRoot, file: File) => {
    const headings = tree.children.filter((node): node is Heading => node.type === "heading")
    const title = headings.find((h) => h.depth === 1) ?? headings[0]
    ;(file.data as TitleData).titleOffset = title?.position?.start.offset
  }
}

/** 文字か数字（和文も含む） */
const wordChar = /[\p{L}\p{N}]/u

type Parent = { children: unknown[] }

/** remark-math のインラインの数式（mdast-util-math の型は直に依存しないので、使う所だけ） */
type InlineMath = Literal & { type: "inlineMath"; data?: { hChildren?: { type: string; value?: string }[] } }

/**
 * インラインの数式（$…$）を、GitHub が数式にするものだけに絞る。remark-math は $ の前後を見ないので、
 * 開きの $ の前か閉じの $ の後が文字・数字のものと、開きの直後が空白のものは、元の文字に戻す（中は Markdown として読み直さない）。
 * $$…$$ はそのまま。$`…`$ は、remark-math が中に残す ` を外す
 */
function remarkGithubMath() {
  return (tree: MdastRoot, file: File) => {
    const source = typeof file.value === "string" ? file.value : ""
    const walk = (parent: Parent) => {
      parent.children = parent.children.map((child) => {
        const node = child as Partial<InlineMath> & Partial<Parent>
        if (node.type !== "inlineMath") {
          if (Array.isArray(node.children)) walk(node as Parent)
          return node
        }
        const start = node.position?.start.offset
        const end = node.position?.end.offset
        if (start === undefined || end === undefined) return node
        const raw = source.slice(start, end)
        if (raw.startsWith("$$")) return node
        const math =
          !wordChar.test(source[start - 1] ?? "") && !wordChar.test(source[end] ?? "") && !/^\s/.test(raw.slice(1))
        if (!math) return { type: "text", value: raw, position: node.position } satisfies Text
        const value = node.value ?? ""
        if (value.length >= 2 && value.startsWith("`") && value.endsWith("`")) {
          node.value = value.slice(1, -1)
          // remark-rehype は中の文字を data.hChildren から取るので、そこも直す
          const text = node.data?.hChildren?.[0]
          if (text?.type === "text") text.value = node.value
        }
        return node
      })
    }
    walk(tree as Parent)
  }
}

const isHeading = (node: RootContent): node is Element => node.type === "element" && /^h[1-6]$/.test(node.tagName)

/**
 * 見出しに GitHub と同じ id（github-slugger）を付け、島の見出しに出す見出しを本文から外す。
 * 外す見出しも数えてから外すので、同じ文字の見出しの -1・-2 は GitHub と揃う。もう id のある見出し（生の HTML）はそのまま。
 * id には、この後の消毒で user-content- が付く（脚注と同じ）
 */
function rehypeHeadingIds() {
  return (tree: Root, file: File) => {
    const slugger = new GithubSlugger()
    const titleOffset = (file.data as TitleData).titleOffset
    const walk = (parent: Root | Element) => {
      parent.children = parent.children.filter((node) => {
        if (node.type !== "element") return true
        if (isHeading(node)) {
          if (node.properties.id === undefined) node.properties.id = slugger.slug(hastToString(node))
          // 外す見出しは Markdown のルートの直下から選んである。生の HTML（details など）で囲むと hast では子になるので、位置だけで見る
          return titleOffset === undefined || node.position?.start.offset !== titleOffset
        }
        walk(node)
        return true
      })
    }
    walk(tree)
  }
}

/** mermaid の図を置く要素の名前。部品（MarkdownBody）がこの要素を pele の図にする。図の元の文字は source に持つ */
export const mermaidTag = "desk-mermaid"

/** ```mermaid のコード（pre > code.language-mermaid）を、図を置く要素にする。Shiki より前に置く（コードとして色を付けない） */
function rehypeMermaid() {
  return (tree: Root) => {
    const walk = (parent: Root | Element) => {
      for (const [index, node] of parent.children.entries()) {
        if (node.type !== "element") continue
        const code = node.tagName === "pre" ? node.children[0] : undefined
        const classes = code?.type === "element" && code.tagName === "code" ? code.properties.className : undefined
        if (code && Array.isArray(classes) && classes.includes("language-mermaid")) {
          const figure: ElementContent = {
            type: "element",
            tagName: mermaidTag,
            properties: { source: hastToString(code) },
            children: [],
          }
          parent.children[index] = figure
        } else walk(node)
      }
    }
    walk(tree)
  }
}

/** 本文に出てくる mermaid の図の種類を、描く前に読んでおく（部品はその場で描くので、読み終えた形にする） */
async function loadDiagrams(tree: Root) {
  const types = new Set<string>()
  const walk = (parent: Root | Element) => {
    for (const node of parent.children) {
      if (node.type !== "element") continue
      if (node.tagName === mermaidTag) {
        // 種類の分からない図は読むものが無い。pele がその場にエラーを出す
        const type = detectType(String(node.properties.source ?? ""))
        if (type) types.add(type)
      } else walk(node)
    }
  }
  walk(tree)
  await Promise.all([...types].map((type) => loadDiagram(type as Parameters<typeof loadDiagram>[0])))
}

/** 子孫に、条件に合う要素があるか */
function some(parent: Root | Element, test: (node: Element) => boolean): boolean {
  return parent.children.some((node) => node.type === "element" && (test(node) || some(node, test)))
}

const classesOf = (node: Element) => (Array.isArray(node.properties.className) ? node.properties.className : [])

/** 数式（remark-math と ```math が出す code.language-math） */
const isMath = (node: Element) => node.tagName === "code" && classesOf(node).includes("language-math")

/** コードの塊（pre > code.language-…）の言語の名前。言語を書いていなければ undefined */
function languageOf(node: Element): string | undefined {
  const code = node.tagName === "pre" ? node.children[0] : undefined
  if (code?.type !== "element" || code.tagName !== "code") return undefined
  const name = classesOf(code).find((c) => String(c).startsWith("language-"))
  return name === undefined ? undefined : String(name).slice("language-".length)
}

/** 言語を書いたコードの塊 */
const isCodeWithLanguage = (node: Element) => languageOf(node) !== undefined

/** 言語を書いたコードの塊を全部 */
function codeBlocks(parent: Root | Element, out: Element[] = []): Element[] {
  for (const node of parent.children) {
    if (node.type !== "element") continue
    if (isCodeWithLanguage(node)) out.push(node)
    else codeBlocks(node, out)
  }
  return out
}

/**
 * 色を後から付けるコードの塊の印（pre の data-highlight 属性）。本文を先に出し、塊を描いた後で CodeBlock が色を付ける
 */
export const highlightLater = "data-highlight"

/**
 * 数式（KaTeX）とコードの色（Shiki）。どちらも大きいので、本文にあるときだけ読む（rich.ts）。
 * 数式を先にする（```math の pre をコードとして色付けしない）。
 * コードの色は、どの言語もすぐ付けられるときだけここで付ける。初めての言語があれば（色の決まりの組み立てに 0.1〜0.2 秒かかる）、
 * 本文を待たせず、塊に印を付けて、描いた後で付ける
 */
function rehypeRich() {
  return async (tree: Root, file: unknown) => {
    const math = some(tree, isMath)
    if (!math && !some(tree, isCodeWithLanguage)) return
    const rich = await import("./rich")
    if (math) rich.katex(tree, file)
    const blocks = codeBlocks(tree)
    if (blocks.length === 0) return
    const languages = [...new Set(blocks.map((pre) => languageOf(pre)!))]
    if (rich.ready(languages)) await rich.highlight(tree, languages)
    else for (const pre of blocks) pre.properties.dataHighlight = ""
  }
}

/**
 * コード 1 つに色を付けた塊（pre.shiki）。本文の木にするときと同じ形で作る。色を付けられない言語なら undefined。
 * 印（highlightLater）の付いた塊を、描いた後で CodeBlock が色を付けるのに使う
 */
export async function highlightCode(language: string, text: string): Promise<Element | undefined> {
  const pre: Element = {
    type: "element",
    tagName: "pre",
    properties: {},
    children: [
      { type: "element", tagName: "code", properties: { className: [`language-${language}`] }, children: [{ type: "text", value: text }] },
    ],
  }
  const root: Root = { type: "root", children: [pre] }
  const rich = await import("./rich")
  await rich.highlight(root, [language])
  // Shiki は塊を、pre.shiki 1 つを持つ root に置き換える
  const out = root.children[0] as Root | RootContent | undefined
  const colored = out?.type === "root" ? out.children[0] : out
  return colored?.type === "element" && colored.tagName === "pre" && colored !== pre ? colored : undefined
}

/** 消毒の許可リスト。既定から、href の scheme だけを図と同じ並びにする */
const sanitizeSchema = { ...defaultSchema, protocols: { ...defaultSchema.protocols, href: urlSchemes } }

const processor = unified()
  .use(remarkParse)
  .use(remarkGfm)
  .use(remarkMath)
  .use(remarkGithubMath)
  .use(remarkFindTitle)
  // 生の HTML は、次の rehype-raw で要素にする。脚注の見出しと戻るリンクの言葉は日本語にする
  .use(remarkRehype, {
    allowDangerousHtml: true,
    footnoteLabel: "脚注",
    footnoteBackLabel: (referenceIndex, rereferenceIndex) =>
      `本文の ${referenceIndex + 1}${rereferenceIndex > 1 ? `-${rereferenceIndex}` : ""} に戻る`,
  })
  .use(rehypeRaw)
  .use(rehypeHeadingIds)
  // 既定の許可リストに、href の scheme だけ図・右の列と同じ並び（urlSchemes）を許す。script・style・iframe・class・style 属性・on… は消え、
  // href は urlSchemes と相対、src は http・https と相対だけが残る。id・name には user-content- が付く。
  // code の language-… の class は残る（数式の language-math、コードの言語）
  .use(rehypeSanitize, sanitizeSchema)
  .use(rehypeMermaid)
  .use(rehypeRich)

/** Markdown の本文（frontmatter を外したもの）を、見出しを外し、消毒し、数式・コードの色・図の種類を読み終えた hast にする */
export async function markdownToHast(markdown: string): Promise<Root> {
  const tree = await processor.run(processor.parse(markdown), markdown)
  await loadDiagrams(tree)
  return tree
}
