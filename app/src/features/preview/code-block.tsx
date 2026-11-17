import type { JSX } from "@solidjs/web"
import { createSignal, omit, onCleanup } from "solid-js"
import { css, cx } from "../../../styled-system/css"
import { iconButton } from "../../../styled-system/recipes"
import { Check, Copy } from "../../components/icons"

/** 写したことを示す印を出しておく間（ミリ秒） */
const copiedFor = 1500

// 写すボタンはコードの右上に重ねる。コードの上にポインタかフォーカスがあるときだけ見せる（タブの閉じるボタンと同じ出し方）。
// 角はコードの塊（radii.surface の 8px）と同心にする。塊の内側 4px に置くので 8 - 4 = 4px（radii.inline）。
// 地は透かさず、島の色をそのまま塗る（surface.solid。下のコードの文字が透けない）
const block = css({ position: "relative" })
const button = css({
  position: "absolute",
  top: "1",
  insetInlineEnd: "1",
  borderRadius: "inline",
  bg: "surface.solid",
  opacity: "[0]",
  _hover: { bg: "surface.solid" },
  ":is(:hover, :focus-within) > &": { opacity: "[1]" },
})

export type CodeBlockProps = Record<string, unknown> & { class?: string; children?: JSX.Element }

/**
 * 本文のコードの塊（pre）。右上に、中の文字を写す（クリップボードに入れる）ボタンを出す。
 * 写す文字は pre の文字（末尾の改行 1 つは外す）。写せたら少しの間、印をチェックにする
 */
export function CodeBlock(props: CodeBlockProps) {
  let pre: HTMLPreElement | undefined
  const [copied, setCopied] = createSignal(false)
  let timer: ReturnType<typeof setTimeout> | undefined
  onCleanup(() => clearTimeout(timer))

  const copy = () => {
    const text = (pre?.textContent ?? "").replace(/\n$/, "")
    navigator.clipboard.writeText(text).then(
      () => {
        setCopied(true)
        clearTimeout(timer)
        timer = setTimeout(() => setCopied(false), copiedFor)
      },
      (e: unknown) => console.error(e),
    )
  }

  const rest = omit(props, "children")
  return (
    <div class={block}>
      <pre ref={pre} {...rest}>
        {props.children}
      </pre>
      <button type="button" aria-label={copied() ? "写した" : "コードを写す"} class={cx(iconButton(), button)} onClick={copy}>
        {copied() ? <Check /> : <Copy />}
      </button>
    </div>
  )
}
