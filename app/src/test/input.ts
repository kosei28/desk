import { separator } from "@ui-sdk/core/separator"
import { mount } from "@ui-sdk/core/testing"

// UI SDK は人の操作でない event（isTrusted が偽）を無視するので、Testing Library の fireEvent では押せない。
// @ui-sdk/core/testing の mount が返す click と keys は、人の操作として扱われる event を送る。
// 公開の入口はこれだけなので、空の separator を mount して借りる。押す先は mount の外の要素でもよい
const borrowed = mount(separator, {}, "<root></root>")
const { click, keys } = borrowed
borrowed.unmount()

export { click, keys }
