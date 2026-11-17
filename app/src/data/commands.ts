// Rust のコマンド（app/src-tauri/src/desk.rs）を呼ぶ層。webview がツリーを読む入口はここだけ。

import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import type { Index, Link } from "./types"

/** 開いているツリーの索引。ツリーを開いていなければ null */
export const getIndex = () => invoke<Index | null>("index")

/** フォルダーを選ぶ窓を出し、選んだツリーを開いて覚える。選ばずに閉じたら何も変わらない */
export const chooseRoot = () => invoke<void>("choose_root")

/** ツリーが変わる（変更が落ち着く）たびに handler を呼ぶ */
export const onTreeChanged = (handler: () => void): Promise<UnlistenFn> => listen("tree-changed", handler)

/** 開いているツリーの、ルートからのパスの Markdown の本文（frontmatter を外したもの）。索引にある .md だけを読める */
export const getBody = (path: string) => invoke<string>("body", { path })

/**
 * 本文の中の、core が読まないリンク（mermaid の図の click の先）を、core の本文のリンクと同じ決まりで解く。
 * from はリンクを書いたファイルのルートからのパス。先があるかは見ない（索引で見る）
 */
export const resolveLink = (from: string, raw: string) => invoke<Link>("resolve_link", { from, raw })

/** 外の URL（url-schemes.ts の urlSchemes）を既定のアプリで開く。ほかの scheme は Rust の側で断る */
export const openUrl = (url: string) => invoke<void>("open_url", { url })
