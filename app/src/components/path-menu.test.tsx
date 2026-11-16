import { render, screen } from "@solidjs/testing-library"
import { createMenuController } from "@ui-sdk/solid/menu"
import { flush } from "solid-js"
import { describe, expect, it, vi } from "vitest"
import { click } from "../test/input"
import { PathMenu, type PathAction } from "./path-menu"

const actions: PathAction[] = ["open-in-new-tab", "close-tab", "copy-path"]

function renderMenu() {
  const onAction = vi.fn()
  const controller = createMenuController<string>()
  render(() => (
    <PathMenu actions={actions} onAction={onAction} controller={controller}>
      <button type="button" data-path="tasks/a.md">
        a
      </button>
      <button type="button" data-path="tasks/b.md">
        b
      </button>
    </PathMenu>
  ))
  return { onAction, controller }
}

const marked = () => [...document.querySelectorAll<HTMLElement>("[data-menu-open]")].map((el) => el.dataset.path)

// 右クリックと Shift+F10 は、ブラウザが出す contextmenu の event で開く。jsdom では人の操作として出せないので、
// 開くところは controller で代える
describe("PathMenu", () => {
  it("開く・閉じると、コピーの間に区切りの線を引く", async () => {
    const { controller } = renderMenu()
    controller.open({ payload: "tasks/a.md" })
    flush()
    const menu = await screen.findByRole("menu")
    expect([...menu.children].map((el) => el.getAttribute("role") ?? el.tagName)).toEqual([
      "menuitem",
      "menuitem",
      "separator",
      "menuitem",
    ])
  })

  it("開いている間、どの要素のメニューかを data-menu-open で示し、閉じると外す", async () => {
    const { controller } = renderMenu()
    controller.open({ payload: "tasks/b.md" })
    flush()
    await screen.findByRole("menu")
    expect(marked()).toEqual(["tasks/b.md"])
    controller.close()
    flush()
    expect(marked()).toEqual([])
  })

  it("項目を選ぶと、そのパスと項目で onAction を呼ぶ", async () => {
    const { onAction, controller } = renderMenu()
    controller.open({ payload: "tasks/a.md" })
    flush()
    await click(await screen.findByRole("menuitem", { name: "パスをコピー" }))
    expect(onAction).toHaveBeenCalledWith("copy-path", "tasks/a.md")
  })
})
