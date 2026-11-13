import { clearMocks, mockIPC } from "@tauri-apps/api/mocks"
import { afterEach, describe, expect, it } from "vitest"
import { getBody } from "./commands"

afterEach(() => clearMocks())

describe("getBody", () => {
  it("パスを渡して body を呼び、本文の文字列を返す", async () => {
    const calls: unknown[] = []
    mockIPC((cmd, args) => {
      calls.push([cmd, args])
      if (cmd === "body") return "# 作業机\n"
    })
    await expect(getBody("README.md")).resolves.toBe("# 作業机\n")
    expect(calls).toEqual([["body", { path: "README.md" }]])
  })

  it("Rust の側が断ったら、その説明で失敗する", async () => {
    mockIPC(() => {
      throw "../outside.md は索引にある Markdown のファイルではない"
    })
    await expect(getBody("../outside.md")).rejects.toBe("../outside.md は索引にある Markdown のファイルではない")
  })
})
