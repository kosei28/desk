// 数式（KaTeX）とコードの色（Shiki）の段。どちらも大きいので、使う本文が来たときに markdown.ts が読む

// 数式の書体と組み方。書体はアプリの中のファイルから読む
import "katex/dist/katex.min.css"
import rehypeShikiFromHighlighter from "@shikijs/rehype/core"
import type { Root } from "hast"
import rehypeKatex from "rehype-katex"
import { createBundledHighlighter } from "shiki/core"
import { createJavaScriptRegexEngine } from "shiki/engine/javascript"
import { bundledLanguages } from "shiki/langs"

type Transform = (tree: Root, file?: unknown) => unknown

/** $…$・$$…$$・```math（code.language-math）を KaTeX の数式にする。書き損じは KaTeX がその場に赤い文字で出す */
export const katex = rehypeKatex() as Transform

/**
 * コードの色を付ける Shiki。言語は、本文に出てきたときに読む（全部を最初に読まない）。
 * 正規表現は JS のもの（WebAssembly を読まないので、CSP に wasm を許さなくてよい）。書けない形の規則は飛ばす
 */
const createHighlighter = createBundledHighlighter({
  langs: bundledLanguages,
  themes: {
    "github-light": () => import("shiki/themes/github-light.mjs"),
    "github-dark": () => import("shiki/themes/github-dark.mjs"),
  },
  engine: () => createJavaScriptRegexEngine({ forgiving: true }),
})

let shiki: Promise<Transform> | undefined

/** Shiki を作る。作れなかったら覚えず、次の本文でもう一度作る */
function loadShiki() {
  const loading = createHighlighter({ themes: ["github-light", "github-dark"], langs: [] }).then(
    (highlighter) =>
      rehypeShikiFromHighlighter(highlighter, {
        themes: { light: "github-light", dark: "github-dark" },
        defaultColor: false,
        lazy: true,
        onError: () => {},
      }) as Transform,
  )
  loading.catch(() => {
    if (shiki === loading) shiki = undefined
  })
  return loading
}

/**
 * 言語を書いたコード（pre > code.language-…）に色を付ける。色はライトとダークの両方を CSS 変数で持ち、
 * どちらを使うかは prose のレシピが決める（明暗が替わっても描き直さない）。読めない言語は色を付けずに残す
 */
export async function highlight(tree: Root) {
  shiki ??= loadShiki()
  await (await shiki)(tree)
}
