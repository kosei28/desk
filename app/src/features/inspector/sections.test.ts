import { flush } from "solid-js"
import { describe, expect, it } from "vitest"
import { createSectionMemory } from "./sections"

describe("createSectionMemory", () => {
  it("どれも開いて始め、見出しの言葉ごとに開け閉めを覚える", () => {
    const sections = createSectionMemory()
    expect(sections.isOpen("リンク")).toBe(true)
    sections.setOpen("他からのリンク", false)
    flush()
    expect(sections.isOpen("他からのリンク")).toBe(false)
    expect(sections.isOpen("リンク")).toBe(true)
    sections.setOpen("他からのリンク", true)
    flush()
    expect(sections.isOpen("他からのリンク")).toBe(true)
  })
})
