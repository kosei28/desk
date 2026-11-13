// Rust のコマンド（app/src-tauri/src/desk.rs）を呼ぶ層。webview がツリーを読む入口はここだけ。

import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import type { Index } from "./types"

/** 開いているツリーの索引。ツリーを開いていなければ null */
export const getIndex = () => invoke<Index | null>("index")

/** フォルダーを選ぶ窓を出し、選んだツリーを開いて覚える。選ばずに閉じたら何も変わらない */
export const chooseRoot = () => invoke<void>("choose_root")

/** ツリーが変わる（変更が落ち着く）たびに handler を呼ぶ */
export const onTreeChanged = (handler: () => void): Promise<UnlistenFn> => listen("tree-changed", handler)
