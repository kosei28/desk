// 本文のリンクと画像の先を、core の答え（Doc の links）から引く。解決の決まり（../ の扱い・アンカー）は TS に持たない。
// 本文の href・src は、Markdown のリンクなら remark が URL として整えたもの（日本語や空白が %xx になる）、
// 生の HTML ならそのままなので、core の raw と、raw を同じく整えたものの両方と照らす。

import { normalizeUri } from "micromark-util-sanitize-uri"
import type { Doc, Link } from "../../data/types"

/** 本文のリンクを押したときにすること */
export type LinkAction =
  /** ツリーの中のファイルを新しいタブで開く（開いているならそのタブに移る）。anchor があれば、その id へ送る */
  | { kind: "open"; path: string; anchor?: string }
  /** 外の URL を既定のアプリで開く */
  | { kind: "url"; url: string }
  /** 同じ本文の中の id へ送る（消毒で id に付いた user-content- は付けたまま） */
  | { kind: "anchor"; id: string }
  /** 何もしない（切れたリンク、ツリーの外、開けない scheme） */
  | { kind: "none" }

/** 消毒（rehype-sanitize）が id・name の頭に付ける文字 */
const clobberPrefix = "user-content-"

/** core の本文のリンクのうち、本文の href・src と同じ先を書いたもの */
function bodyLink(doc: Doc, value: string): Link | undefined {
  const trimmed = value.trim()
  return doc.links.find(
    (link) => link.key === "body" && (link.raw === trimmed || normalizeUri(link.raw) === trimmed),
  )
}

function decode(s: string): string {
  try {
    return decodeURIComponent(s)
  } catch {
    return s
  }
}

/** # の後ろ（見出しや脚注の名前）を、本文の要素の id にする。消毒が付けた user-content- を足す */
const anchorId = (anchor: string) => clobberPrefix + decode(anchor)

/** 書いてある先（href）と、core がそれを解いたリンク（無ければ undefined）から、押したときにすることを決める */
function actionOf(href: string, link: Link | undefined, byPath: ReadonlyMap<string, Doc>): LinkAction {
  if (href.startsWith("#")) return { kind: "anchor", id: anchorId(href.slice(1)) }
  if (link?.form === "relative") {
    if (!byPath.has(link.target)) return { kind: "none" }
    return link.anchor ? { kind: "open", path: link.target, anchor: anchorId(link.anchor) } : { kind: "open", path: link.target }
  }
  if (/^(https?|mailto):/i.test(href.trim())) return { kind: "url", url: href.trim() }
  return { kind: "none" }
}

/** 本文のリンク（a の href）を押したときにすること */
export function linkAction(doc: Doc, href: string, byPath: ReadonlyMap<string, Doc>): LinkAction {
  return actionOf(href, href.startsWith("#") ? undefined : bodyLink(doc, href), byPath)
}

/** 書いてある先のうち、パスとして解くもの（# だけのものと、scheme の付いた URL は解かない） */
const isPath = (href: string) => !href.startsWith("#") && !/^[a-z][a-z0-9+.-]*:/i.test(href)

/**
 * 図の中のリンク（mermaid の click の先）を押したときにすること。core の索引には無いので、
 * パスはその場で core に解いてもらう（resolve。解く決まりは core だけが持つ）。ほかは本文のリンクと同じ
 */
export async function diagramLinkAction(
  doc: Doc,
  href: string,
  byPath: ReadonlyMap<string, Doc>,
  resolve: (from: string, raw: string) => Promise<Link>,
): Promise<LinkAction> {
  const raw = href.trim()
  return actionOf(raw, isPath(raw) ? await resolve(doc.path, raw) : undefined, byPath)
}

/**
 * 本文の画像の先（img の src・source の srcset の 1 つ）を、webview が読む URL にする。
 * ローカルのファイル（相対パス。ツリーの外へ出るものも、/ 始まり・~ 始まりも）は Rust の scheme から、http・https はそのまま読む。
 * 画像の形のファイルかどうかは Rust が見る。ほかの先（読めない scheme・core の読んだリンクに無いもの）は読まない。
 * http は、本番の形では CSP の img-src に入れていないので出ない。出すための手は入れない
 */
export function imageSrc(doc: Doc, src: string): string | undefined {
  const link = bodyLink(doc, src)
  if (link?.form === "relative" || link?.form === "absolute" || link?.form === "home") return imageUrl(link.target)
  if (/^https?:/i.test(src.trim())) return src.trim()
  return undefined
}

/**
 * ローカルのファイルを、Rust が配る独自の scheme の URL にする。パスはルートからのパス（外なら ../ 始まり）か、/ 始まり・~ 始まり。
 * パス全体を 1 つの段として %xx にする（/ も）。段に分けると、webview が URL の .. を畳んでしまう
 */
export function imageUrl(path: string): string {
  return `desk://localhost/${encodeURIComponent(path)}`
}

/** HTML の空白（ASCII whitespace） */
const isSpace = (c: string) => c === "\t" || c === "\n" || c === "\f" || c === "\r" || c === " "

/** srcset の候補（URL と、後ろの記述子）。HTML の仕様の「srcset 属性を読む」の、URL を取り出すところ（core と同じ読み方） */
export function srcsetCandidates(value: string): { url: string; descriptor: string }[] {
  const out: { url: string; descriptor: string }[] = []
  let i = 0
  for (;;) {
    while (i < value.length && (isSpace(value[i]) || value[i] === ",")) i++
    if (i === value.length) return out
    const start = i
    while (i < value.length && !isSpace(value[i])) i++
    const word = value.slice(start, i)
    const url = word.replace(/,+$/, "")
    if (url.length < word.length) {
      out.push({ url, descriptor: "" })
      continue
    }
    // 記述子（2x・100w など）を、括弧の外のカンマまで読む
    const from = i
    let inParens = false
    while (i < value.length) {
      const c = value[i]
      if (c === "(") inParens = true
      else if (c === ")") inParens = false
      else if (c === "," && !inParens) break
      i++
    }
    out.push({ url, descriptor: value.slice(from, i).trim() })
  }
}
