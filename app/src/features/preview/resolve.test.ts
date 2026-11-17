import { describe, expect, it, vi } from "vitest"
import type { Doc, Link } from "../../data/types"
import { diagramLinkAction, imageUrl, srcsetCandidates } from "./resolve"

describe("srcsetCandidates", () => {
  it("core と同じく、候補の URL と記述子を読む（URL の中のカンマ・括弧の中のカンマ・空の候補）", () => {
    expect(srcsetCandidates(" a.png 1x, b,c.png 2x,d.png,, e.png (1, 2) 100w,f.png")).toEqual([
      { url: "a.png", descriptor: "1x" },
      { url: "b,c.png", descriptor: "2x" },
      { url: "d.png", descriptor: "" },
      { url: "e.png", descriptor: "(1, 2) 100w" },
      { url: "f.png", descriptor: "" },
    ])
    expect(srcsetCandidates(" , ")).toEqual([])
  })
})

describe("imageUrl", () => {
  it("パス全体を 1 つの段として %xx にする（/ も。.. を webview に畳ませない）", () => {
    expect(imageUrl("projects/a/artifacts/assets/50%#1 見本.png")).toBe(
      "desk://localhost/projects%2Fa%2Fartifacts%2Fassets%2F50%25%231%20%E8%A6%8B%E6%9C%AC.png",
    )
    expect(new URL(imageUrl("../photos/a.png")).pathname).toBe("/..%2Fphotos%2Fa.png")
    expect(imageUrl("~/Pictures/a.png")).toBe("desk://localhost/~%2FPictures%2Fa.png")
  })
})

describe("diagramLinkAction", () => {
  const from = { path: "projects/a/artifacts/flow.md", links: [] } as unknown as Doc
  const task = "projects/a/tasks/2026-01-02-tidy.md"
  const byPath = new Map([[task, { path: task } as Doc]])
  // core の resolve_link の代わり。呼ばれた先だけを、core が返す形で返す
  const resolve = vi.fn(async (_from: string, raw: string): Promise<Link> => {
    const [path, anchor = ""] = raw.split("#")
    const target = path === "../tasks/2026-01-02-tidy.md" ? task : "projects/a/missing.md"
    return { key: "body", text: "", raw, line: 0, start: null, form: "relative", target, anchor } as Link
  })

  it("パスは core に解いてもらい、ツリーにあれば開く（# の先は本文の id にする）", async () => {
    expect(await diagramLinkAction(from, "../tasks/2026-01-02-tidy.md", byPath, resolve)).toEqual({ kind: "open", path: task })
    expect(await diagramLinkAction(from, " ../tasks/2026-01-02-tidy.md#手順 ", byPath, resolve)).toEqual({
      kind: "open",
      path: task,
      anchor: "user-content-手順",
    })
    expect(resolve).toHaveBeenCalledWith("projects/a/artifacts/flow.md", "../tasks/2026-01-02-tidy.md#手順")
    expect(await diagramLinkAction(from, "../missing.md", byPath, resolve)).toEqual({ kind: "none" })
  })

  it("外の URL と # だけの先は、core に聞かずに本文のリンクと同じく扱う", async () => {
    resolve.mockClear()
    expect(await diagramLinkAction(from, "https://example.com/a", byPath, resolve)).toEqual({
      kind: "url",
      url: "https://example.com/a",
    })
    expect(await diagramLinkAction(from, "#まとめ", byPath, resolve)).toEqual({ kind: "anchor", id: "user-content-まとめ" })
    expect(await diagramLinkAction(from, "ftp://example.com/a", byPath, resolve)).toEqual({ kind: "none" })
    expect(resolve).not.toHaveBeenCalled()
  })
})
