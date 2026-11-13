import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

const invoke = vi.hoisted(() => vi.fn(() => Promise.resolve()))
vi.mock("@tauri-apps/api/core", () => ({ invoke }))

import { dragRegion } from "./drag-region"

function press(target: Element, type: "mousedown" | "mouseup", detail: number, at = { x: 10, y: 10 }, button = 0) {
  target.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, detail, button, clientX: at.x, clientY: at.y }))
}

describe("dragRegion", () => {
  let band: HTMLDivElement
  let child: HTMLParagraphElement

  beforeEach(() => {
    invoke.mockClear()
    band = document.createElement("div")
    child = document.createElement("p")
    band.append(child)
    document.body.append(band)
    dragRegion(band)
  })
  afterEach(() => band.remove())

  it("帯を 1 回押すとウィンドウを動かす", () => {
    press(band, "mousedown", 1)
    press(band, "mouseup", 1)
    expect(invoke.mock.calls).toEqual([["plugin:window|start_dragging"]])
  })

  it("ダブルクリックは 2 回目を離したときにシステム設定どおりのアクションを 1 回だけ送る", () => {
    press(band, "mousedown", 1)
    press(band, "mouseup", 1)
    press(band, "mousedown", 2)
    expect(invoke.mock.calls).toEqual([["plugin:window|start_dragging"]])
    press(band, "mouseup", 2)
    expect(invoke.mock.calls).toEqual([["plugin:window|start_dragging"], ["titlebar_double_click"]])
    expect(invoke.mock.calls.flat()).not.toContain("plugin:window|internal_toggle_maximize")
  })

  it("2 回目を押してから動かしたら送らない", () => {
    press(band, "mousedown", 2, { x: 10, y: 10 })
    press(band, "mouseup", 2, { x: 14, y: 10 })
    expect(invoke).not.toHaveBeenCalled()
  })

  it("帯の中の子の要素や右ボタンでは何もしない", () => {
    press(child, "mousedown", 1)
    press(child, "mousedown", 2)
    press(child, "mouseup", 2)
    press(band, "mousedown", 1, undefined, 2)
    expect(invoke).not.toHaveBeenCalled()
  })
})
