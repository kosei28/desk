// Markdown の本文を hast（HTML の木）にする。描き方の決まりは desk の「Markdown のレンダリングの技術選定」。
// GFM と生の HTML を要素にし、GitHub とほぼ同じ許可リスト（rehype-sanitize の既定）で消毒する。
// 数式・コードの色・mermaid は、この流れに後から足す（順 5c）。

import GithubSlugger from "github-slugger"
import type { Element, Root, RootContent } from "hast"
import { toString as hastToString } from "hast-util-to-string"
import type { Heading, Root as MdastRoot } from "mdast"
import rehypeRaw from "rehype-raw"
import rehypeSanitize from "rehype-sanitize"
import remarkGfm from "remark-gfm"
import remarkParse from "remark-parse"
import remarkRehype from "remark-rehype"
import { unified } from "unified"

/** 島の見出しに出すので本文から外す見出しの、元の文字の中の位置（hast の position.start.offset と同じ数え方） */
type TitleData = { titleOffset?: number }

/** unified の VFile のうち、ここで使う所（vfile は直に依存しない） */
type File = { data: object }

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

const processor = unified()
  .use(remarkParse)
  .use(remarkGfm)
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
  // 既定の許可リストのまま。script・style・iframe・class・style 属性・on… は消え、
  // href・src は http・https・mailto（a だけ）・相対だけが残る。id・name には user-content- が付く
  .use(rehypeSanitize)

/** Markdown の本文（frontmatter を外したもの）を、見出しを外し、消毒した hast にする */
export function markdownToHast(markdown: string): Root {
  return processor.runSync(processor.parse(markdown))
}
