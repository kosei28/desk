import { invoke } from "@tauri-apps/api/core"

/**
 * 要素をウィンドウを動かす帯にする。`ref` に渡す。
 *
 * 帯そのもの（子の要素ではない）を押したらウィンドウを動かし、ダブルクリックしたら
 * AppKit のタイトルバーと同じアクションを送る（Rust の `titlebar_double_click`。システム設定の
 * 「タイトルバーをダブルクリックして…」に従う）。ダブルクリックは AppKit と同じく、
 * 2 回目を離したときに、押した位置から動いていなければ送る。
 *
 * Tauri の `data-tauri-drag-region` は使わない。ダブルクリックで設定を見ずに最大化を切り替え、
 * その最大化では webview の描画が窓の動きに追いつかないため。
 */
export function dragRegion(element: HTMLElement): void {
  let secondPress: { x: number; y: number } | null = null

  element.addEventListener("mousedown", (event) => {
    if (event.button !== 0 || event.target !== element) return
    if (event.detail === 1) {
      // 文字を選ぶカーソルにしない
      event.preventDefault()
      void invoke("plugin:window|start_dragging")
    } else if (event.detail === 2) {
      secondPress = { x: event.clientX, y: event.clientY }
    }
  })

  element.addEventListener("mouseup", (event) => {
    const pressed = secondPress
    secondPress = null
    if (event.button !== 0 || event.detail !== 2 || event.target !== element || pressed === null) return
    if (event.clientX !== pressed.x || event.clientY !== pressed.y) return
    void invoke("titlebar_double_click")
  })
}
