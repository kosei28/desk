import { describe, expect, it } from "vitest"
import { imageUrl, srcsetCandidates } from "./resolve"

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
