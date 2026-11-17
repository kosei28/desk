import type { Root } from "hast"
import { createEffect, createMemo, createSignal, Match, Show, Switch } from "solid-js"
import { css } from "../../../styled-system/css"
import { prose } from "../../../styled-system/recipes"
import { getBody } from "../../data/commands"
import type { Doc } from "../../data/types"
import { markdownToHast } from "./markdown"
import { MarkdownBody, scrollBox, scrollToTop } from "./markdown-body"
import { imageUrl } from "./resolve"

/** 島に画像として出すファイルの拡張子 */
const imageExtensions = new Set(["png", "jpg", "jpeg", "gif", "webp", "avif", "svg", "bmp", "ico"])

/** 島に画像として出すファイルか（拡張子で見る。Rust の scheme が配るのも同じ拡張子のもの） */
export function isImage(doc: Doc): boolean {
  const dot = doc.path.lastIndexOf(".")
  return dot !== -1 && imageExtensions.has(doc.path.slice(dot + 1).toLowerCase())
}

export type FilePreviewProps = {
  doc: Doc
  /** 島の見出し（見出しが無ければファイル名） */
  title: string
  byPath: ReadonlyMap<string, Doc>
  onOpen: (path: string, anchor?: string) => void
  onOpenUrl: (url: string) => void
  /**
   * 中身を描いたら送る先（本文の要素の id）。ほかのファイルのリンク（file.md#見出し）で開いたとき。頼むたびに別のオブジェクトにする。
   * 島に出ている（隠していてもよい）ときだけ渡す。隠れたタブ（display: none）では位置を測れない
   */
  anchor?: { id: string }
  /** anchor へ送り終えた（当たらなければ上へ送った） */
  onAnchorDone?: () => void
}

/**
 * 島の中身。見出し（h1）と description の下に、Markdown なら本文、画像なら画像を出す。ほかのファイルは見出しだけ。
 * 本文は doc が変わるたび（ツリーが変わって索引をもらい直すたび）に読み直す。中身が同じなら描き直さない。
 * 本文は、数式・コードの色・図の種類まで読み終えた木にしてから描く（描いた後で中身の高さが変わらない）。
 */
export function FilePreview(props: FilePreviewProps) {
  let root: HTMLDivElement | undefined
  const [body, setBody] = createSignal<string>()
  // body を木にしたもの。body が変わったときだけ作り直す
  const [tree, setTree] = createSignal<Root>()
  // 本文を読めなかった。描くものはもう来ないので、anchor は上へ送って終える
  const [failed, setFailed] = createSignal(false)

  createEffect(
    () => props.doc,
    (doc) => {
      if (!doc.is_md) return
      // 読み直しが重なったら、最後に頼んだものの結果だけを使う
      let stale = false
      getBody(doc.path).then(
        (text) => !stale && setBody(text),
        (e: unknown) => {
          if (stale) return
          console.error(e)
          setFailed(true)
        },
      )
      return () => {
        stale = true
      }
    },
  )

  createEffect(body, (text) => {
    if (text === undefined) return
    // 木にするのが重なったら、最後の本文の木だけを使う
    let stale = false
    markdownToHast(text).then(
      (root) => !stale && setTree(root),
      (e: unknown) => {
        if (stale) return
        console.error(e)
        setFailed(true)
      },
    )
    return () => {
      stale = true
    }
  })

  // 中身を描き終えたか（Markdown なら本文が届いて木にして描いた。空の本文も、読めなかったときも終わり）
  const drawn = createMemo(() => !props.doc.is_md || tree() !== undefined || failed())

  // anchor があれば、描いたのと同じ流れの中で（絵を出す前に）その id へ送る。当たらなければ上へ。
  // 次のフレームまで待つと、上から（または前の位置で）描いた絵が 1 枚出てチラつく
  createEffect(
    () => (drawn() ? props.anchor : undefined),
    (anchor) => {
      if (!anchor || !root) return
      const target = root.querySelector(`[id="${CSS.escape(anchor.id)}"]`)
      if (target) scrollToTop(target)
      else scrollBox(root)?.scrollTo?.({ top: 0 })
      props.onAnchorDone?.()
    },
  )

  // 見出しと description の組と本文の間は、本文の節と同じ間（h2 の前の 40px）
  return (
    <div ref={root} class={css({ display: "flex", flexDirection: "column", gap: "10" })}>
      <header class={css({ display: "grid", gap: "2" })}>
        <h1 class={css({ textStyle: "title" })}>{props.title}</h1>
        <Show when={props.doc.description}>
          {(description) => <p class={css({ textStyle: "ui" })}>{description()}</p>}
        </Show>
      </header>
      <Switch>
        <Match when={props.doc.is_md && tree()}>
          {(root) => (
            <MarkdownBody
              doc={props.doc}
              tree={root()}
              byPath={props.byPath}
              onOpen={props.onOpen}
              onOpenUrl={props.onOpenUrl}
            />
          )}
        </Match>
        <Match when={isImage(props.doc)}>
          <div class={prose()}>
            <img src={imageUrl(props.doc.path)} alt={props.doc.path.slice(props.doc.path.lastIndexOf("/") + 1)} />
          </div>
        </Match>
      </Switch>
    </div>
  )
}
