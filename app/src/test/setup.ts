import { cleanup } from "@solidjs/testing-library"
import { afterEach } from "vitest"

declare global {
  var __UI_SDK_STRICT__: boolean | undefined
}
// UI SDK の開発時の警告でテストを落とす
globalThis.__UI_SDK_STRICT__ = true

// jsdom に無いか、あっても足りない DOM の API。UI SDK の skill（references/testing.md）の一覧どおりに置く
HTMLElement.prototype.showPopover = function () {}
HTMLElement.prototype.hidePopover = function () {}
Element.prototype.getAnimations = () => []
Element.prototype.scrollIntoView = function () {}
Element.prototype.checkVisibility = () => true
Element.prototype.getClientRects = () => [] as unknown as DOMRectList
document.elementFromPoint = () => null
document.elementsFromPoint = () => []
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
}

afterEach(cleanup)
