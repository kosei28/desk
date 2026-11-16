import type { TreeItemData } from "@ui-sdk/solid/tree"
import { describe, expect, it } from "vitest"
import type { Doc } from "../../data/types"
import { deskTreeItems, fileEntry } from "./desk-tree"

const doc = (path: string, fields: Partial<Doc> = {}): Doc => ({
  path,
  kind: null,
  project: "",
  archived: false,
  project_archived: false,
  asset: false,
  problem: null,
  is_md: path.endsWith(".md"),
  frontmatter: null,
  frontmatter_error: null,
  title: "",
  description: "",
  now: "",
  status: "",
  needs_human: false,
  base: "",
  headings: [],
  links: [],
  ...fields,
})

/** 木を「字下げ + 言葉」の行にする */
const rows = (items: TreeItemData[], depth = 0): string[] =>
  items.flatMap((item) => [`${"  ".repeat(depth)}${item.label}`, ...rows(item.children ?? [], depth + 1)])

describe("deskTreeItems", () => {
  it("どの段も README → フォルダー（名前の順）→ ファイル（見出しの順）→ archive・assets", () => {
    const items = deskTreeItems([
      doc("projects/garden/README.md", { title: "庭の記録" }),
      doc("knowledge/write-short.md", { title: "短く書く" }),
      doc("archive/projects/old/README.md", { title: "古い" }),
      doc("README.md", { title: "作業机" }),
      doc("principles/prefer-structure.md", { title: "起きない形にして直す" }),
      doc("projects/garden/artifacts/beds.md", { title: "畝の配置" }),
      doc("projects/garden/artifacts/assets/beds.png"),
      doc("projects/garden/artifacts/archive.md", { title: "あとで読む" }),
      doc("projects/garden/assets/logo.svg"),
      doc("projects/garden/archive/tasks/2026-01-02-sow.md", { title: "種をまく" }),
    ])
    expect(rows(items)).toEqual([
      "README",
      "knowledge",
      "  短く書く",
      "principles",
      "  起きない形にして直す",
      "projects",
      "  garden",
      "    README",
      "    artifacts",
      "      あとで読む",
      "      畝の配置",
      "      assets",
      "        beds.png",
      "    archive",
      "      tasks",
      "        種をまく",
      "    assets",
      "      logo.svg",
      "archive",
      "  projects",
      "    old",
      "      README",
    ])
  })

  it("日付で始まる名前は新しい順で、日付の無いファイルより先。同じ日は見出しの順", () => {
    const items = deskTreeItems([
      doc("inbox/notes.md", { title: "メモ" }),
      doc("inbox/2026-11-09-b.md", { title: "い" }),
      doc("inbox/2026-11-10-paper.md", { title: "用紙" }),
      doc("inbox/2026-11-09-a.md", { title: "あ" }),
      doc("inbox/2025-12-31-old.md", { title: "去年" }),
    ])
    expect(rows(items)).toEqual(["inbox", "  用紙", "  あ", "  い", "  去年", "  メモ"])
  })

  it("見出しの無いファイルはファイル名で出し、assets の中は見出しがあってもファイル名で出す", () => {
    const items = deskTreeItems([
      doc("wiki/plain.md"),
      doc("wiki/assets/diagram.md", { title: "図の下書き" }),
    ])
    expect(rows(items)).toEqual(["wiki", "  plain.md", "  assets", "    diagram.md"])
  })

  it("値はルートからのパス。フォルダーも / で終わらない", () => {
    const [folder] = deskTreeItems([doc("projects/garden/README.md", { title: "庭" })])
    expect(folder.value).toBe("projects")
    expect(folder.children?.[0].value).toBe("projects/garden")
    expect(folder.children?.[0].children?.[0].value).toBe("projects/garden/README.md")
  })
})

describe("fileEntry", () => {
  it("タスクにだけ status の印と人の印を付ける。決まりに無い status は印を付けない", () => {
    expect(fileEntry(doc("t/2026-11-10-a.md", { kind: "task", status: "todo", needs_human: true }))).toMatchObject({
      status: "todo",
      needsHuman: true,
    })
    expect(fileEntry(doc("t/2026-11-10-b.md", { kind: "task", status: "doing" })).status).toBeUndefined()
    expect(fileEntry(doc("a/b.md", { kind: "artifact", status: "todo", needs_human: true }))).toMatchObject({
      status: undefined,
      needsHuman: false,
    })
  })
})
