import { Fragment, jsx, jsxs } from "@solidjs/h/jsx-runtime"
import type { JSX } from "@solidjs/web"
import type { Root } from "hast"
import { toJsxRuntime } from "hast-util-to-jsx-runtime"
import { createMemo, omit } from "solid-js"
import { prose } from "../../../styled-system/recipes"
import { resolveLink } from "../../data/commands"
import type { Doc } from "../../data/types"
import { CodeBlock } from "./code-block"
import { mermaidTag } from "./markdown"
import { MermaidDiagram } from "./mermaid-diagram"
import { diagramLinkAction, imageSrc, type LinkAction, linkAction, srcsetCandidates } from "./resolve"

export type MarkdownBodyProps = {
  /** 本文を持つファイル。リンクと画像の先を、その links から引く */
  doc: Doc
  /** 本文（frontmatter を外したもの）を markdownToHast で木にしたもの */
  tree: Root
  /** 索引のパスの引き当て。リンクの先がツリーにあるかを見る */
  byPath: ReadonlyMap<string, Doc>
  /** ツリーの中のファイルを開く。anchor（本文の要素の id）があれば、開いた先でそこへ送る */
  onOpen: (path: string, anchor?: string) => void
  /** 外の URL を開く */
  onOpenUrl: (url: string) => void
}

type ElementProps = Record<string, unknown> & { children?: JSX.Element }

/** いちばん近い縦にスクロールする祖先（島） */
export function scrollBox(element: Element): HTMLElement | undefined {
  for (let box = element.parentElement; box; box = box.parentElement) {
    const { overflowY } = getComputedStyle(box)
    if (overflowY === "auto" || overflowY === "scroll") return box
  }
  return undefined
}

/**
 * 本文の中の target を、島の上の端（内側の余白の下）へ送る。
 * scrollIntoView は使わない。外の overflow: hidden の箱（アプリの画面の枠）まで動かし、画面ごとずれる
 */
export function scrollToTop(target: Element) {
  const box = scrollBox(target)
  if (!box) return
  const offset = target.getBoundingClientRect().top - box.getBoundingClientRect().top
  box.scrollTo?.({ top: box.scrollTop + offset - (parseFloat(getComputedStyle(box).paddingTop) || 0) })
}

/**
 * Markdown の本文を、読む形（prose）で描く。見出しは島の見出しに出すので、本文には出さない。
 * リンクは押しても webview の中で移らず、中のファイルは onOpen、外の URL は onOpenUrl、# の先（見出し・脚注）は本文の中へ送る。
 * 画像は、ローカルのファイルなら Rust の scheme から、http・https ならそのまま読む。
 * mermaid の図は pele で描く。図の中のリンクも webview の中では移らず、本文のリンクと同じく開く（先は core に解いてもらう）。
 * コードの塊には、右上に写すボタンを出す。
 */
export function MarkdownBody(props: MarkdownBodyProps) {
  let root: HTMLDivElement | undefined

  const run = (action: LinkAction) => {
    if (action.kind === "open") props.onOpen(action.path, action.anchor)
    else if (action.kind === "url") props.onOpenUrl(action.url)
    else if (action.kind === "anchor") {
      const target = root?.querySelector(`[id="${CSS.escape(action.id)}"]`)
      if (target) scrollToTop(target)
    }
  }

  function BodyLink(link: ElementProps & { href?: string }) {
    const rest = omit(link, "href", "children")
    return (
      <a
        {...rest}
        href={link.href}
        onClick={(event: MouseEvent) => {
          event.preventDefault()
          if (link.href === undefined) return
          run(linkAction(props.doc, link.href, props.byPath))
        }}
      >
        {link.children}
      </a>
    )
  }

  const src = (value: unknown) => {
    if (typeof value !== "string") return undefined
    return imageSrc(props.doc, value)
  }

  const srcset = (value: unknown) => {
    if (typeof value !== "string") return undefined
    const candidates = srcsetCandidates(value).flatMap(({ url, descriptor }) => {
      const resolved = src(url)
      return resolved === undefined ? [] : [descriptor ? `${resolved} ${descriptor}` : resolved]
    })
    return candidates.length ? candidates.join(", ") : undefined
  }

  function BodyImage(image: ElementProps & { src?: string }) {
    const rest = omit(image, "src")
    return <img {...rest} src={src(image.src)} />
  }

  function BodySource(source: ElementProps & { srcset?: string }) {
    const rest = omit(source, "srcset")
    return <source {...rest} srcset={srcset(source.srcset)} />
  }

  function BodyMermaid(figure: ElementProps & { source?: string }) {
    const onLink = (href: string) =>
      void diagramLinkAction(props.doc, href, props.byPath, resolveLink).then(run, (e: unknown) => console.error(e))
    return <MermaidDiagram source={figure.source ?? ""} onLink={onLink} />
  }

  const content = createMemo(() =>
    toJsxRuntime(props.tree, {
      Fragment,
      jsx,
      jsxs,
      elementAttributeNameCase: "html",
      stylePropertyNameCase: "css",
      // 型は React の JSX を前提にしているので、Solid の部品は合わない。props の形は html の名前（class・srcset）
      components: { a: BodyLink, img: BodyImage, source: BodySource, pre: CodeBlock, [mermaidTag]: BodyMermaid } as never,
    }),
  )

  return (
    <div ref={root} class={prose()}>
      {content() as JSX.Element}
    </div>
  )
}
