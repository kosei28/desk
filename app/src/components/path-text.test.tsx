import { render } from "@solidjs/testing-library"
import { describe, expect, it } from "vitest"
import { PathText } from "./path-text"

describe("PathText", () => {
  it("/ の後ろにだけ折れる所を置き、ハイフンとその次の 1 字は折れない塊にする。文字は変えない", () => {
    const path = "~/src/recipe-app-worktrees/meal-planner"
    const { container } = render(() => <PathText path={path} />)
    const root = container.firstElementChild!
    expect(root.textContent).toBe(path)
    expect(root.querySelectorAll("wbr")).toHaveLength(3)
    expect([...root.querySelectorAll("[data-keep]")].map((e) => e.textContent)).toEqual(["-a", "-w", "-p"])
  })

  it("拡張子は前の語と折れない塊にし、行の終わりに拡張子だけが残らないようにする", () => {
    const path = "projects/recipe-app/materials/2026-11-14-fixed-preview-per-browser.md"
    const { container } = render(() => <PathText path={path} />)
    const root = container.firstElementChild!
    expect(root.textContent).toBe(path)
    expect([...root.querySelectorAll("[data-keep]")].map((e) => e.textContent)).toEqual([
      "-a",
      "-1",
      "-1",
      "-f",
      "-p",
      "-p",
      "-browser.md",
    ])
  })
})
