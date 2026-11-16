import { describe, expect, it } from "vitest"
import type { Doc, FrontmatterField, Link } from "../../data/types"
import { inspect, linkIndex, type InspectorFile } from "./inspect"

const doc = (path: string, fields: Partial<Doc> = {}): Doc => {
  const parts = path.split("/")
  const projectArchived = parts[0] === "archive"
  const inProject = parts[projectArchived ? 1 : 0] === "projects"
  const project = inProject ? parts[projectArchived ? 2 : 1] : ""
  return {
    path,
    kind: null,
    project,
    archived: projectArchived || parts.includes("archive"),
    project_archived: projectArchived,
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
  }
}

/** 相対リンク 1 つ。target はルートからのパス（core が解いたもの）、raw は書いてある文字（省けば target） */
const link = (key: string, target: string, raw = target): Link => ({
  key,
  text: "",
  raw,
  line: 0,
  start: null,
  form: "relative",
  target,
  anchor: "",
})

const fm = (entries: Record<string, string | string[] | null>): FrontmatterField[] =>
  Object.entries(entries).map(([key, value]) => ({ key, value }))

const titles = (files: InspectorFile[]) => files.map((file) => (file.place ? `${file.title}（${file.place}）` : file.title))

const P = "projects/recipe-app"
const task = (slug: string, title: string, fields: Partial<Doc> = {}) =>
  doc(`${P}/tasks/${slug}.md`, { kind: "task", title, status: "todo", ...fields })

describe("inspect", () => {
  it("頭は書いてある順に、名前の付いたキーは名前で、決まりに無いキーは綴りのまま出す。空の値は出さない", () => {
    const here = doc(`${P}/library/2026-11-10-scroll.md`, {
      kind: "library",
      frontmatter: fm({
        reviewer: "ichiro",
        url: "https://example.com/infinite-scroll",
        source: "2026-11-10 の勉強会",
        empty: null,
        blank: "",
        tags: ["pagination", "frontend"],
        verified: "2026-11-12",
        description: "無限スクロールの落とし穴",
        aliases: ["a", "b"],
      }),
    })
    const content = inspect(here, linkIndex([here]))
    expect(content.path).toBe(here.path)
    expect(content.head).toEqual([
      { label: "reviewer", value: "ichiro", form: "text" },
      { label: "URL", value: "https://example.com/infinite-scroll", form: "url" },
      { label: "出どころ", value: "2026-11-10 の勉強会", form: "text" },
      { label: "確認日", value: "2026-11-12", form: "text" },
      { label: "概要", value: "無限スクロールの落とし穴", form: "text" },
      { label: "aliases", value: "a, b", form: "text" },
    ])
    expect(content.tags).toEqual(["pagination", "frontend"])
  })

  it("札は status（決まりの値のとき）・needs_human（true のとき）・tags。決まりに無い値は頭に綴りのまま出す", () => {
    const human = task("2026-11-10-decide-paper-size", "用紙", {
      frontmatter: fm({ status: "todo", needs_human: "true", tags: "print" }),
    })
    expect(inspect(human, linkIndex([human]))).toMatchObject({
      status: "todo",
      needsHuman: true,
      tags: ["print"],
      head: [],
    })

    const odd = task("2026-11-11-odd", "変な値", { frontmatter: fm({ status: "waiting", needs_human: "false" }) })
    const content = inspect(odd, linkIndex([odd]))
    expect(content.status).toBeUndefined()
    expect(content.needsHuman).toBe(false)
    expect(content.head).toEqual([{ label: "status", value: "waiting", form: "text" }])
  })

  it("コードは決まった順に出し、ベースブランチは branch の行のコメントから", () => {
    const here = task("2026-11-09-printable-menu", "印刷", {
      frontmatter: fm({
        pr: "https://github.com/example/recipe-app/pull/128",
        worktree: "~/src/recipe-app-worktrees/meal-planner",
        branch: "feature/meal-planner/printable-menu",
        repo: "~/src/recipe-app",
      }),
      base: "feature/meal-planner/all",
    })
    expect(inspect(here, linkIndex([here])).code).toEqual([
      { label: "リポジトリ", value: "~/src/recipe-app", form: "path" },
      { label: "ブランチ", value: "feature/meal-planner/printable-menu", form: "path" },
      { label: "ベースブランチ", value: "feature/meal-planner/all", form: "path" },
      { label: "ワークツリー", value: "~/src/recipe-app-worktrees/meal-planner", form: "path" },
      { label: "PR", value: "https://github.com/example/recipe-app/pull/128", form: "url" },
    ])
  })

  it("親子と依存は、キーごとに このファイルの値 → 逆向き。空のリストは「[]」、切れたリンクは出さない", () => {
    const parent = task("2026-09-07-meal-planner", "献立プランナー")
    const here = task("2026-11-09-printable-menu", "印刷", {
      frontmatter: fm({ parent: "2026-09-07-meal-planner.md", depends_on: [] }),
      links: [link("parent", parent.path, "2026-09-07-meal-planner.md")],
    })
    const child = task("2026-11-11-print-preview", "プレビュー", {
      needs_human: true,
      links: [link("parent", here.path)],
    })
    const later = task("2026-11-12-shopping-list", "買い物リスト", {
      status: "backlog",
      links: [link("depends_on", here.path), link("depends_on", `${P}/tasks/gone.md`)],
    })
    const content = inspect(here, linkIndex([parent, here, child, later]))
    expect(content.family).toEqual([
      { label: "親タスク", files: [{ path: parent.path, title: "献立プランナー", status: "todo", needsHuman: false }] },
      { label: "子タスク", files: [{ path: child.path, title: "プレビュー", status: "todo", needsHuman: true }] },
      { label: "先行タスク", files: [] },
      { label: "後続タスク", files: [{ path: later.path, title: "買い物リスト", status: "backlog", needsHuman: false }] },
    ])
    // parent・depends_on は頭に出さない
    expect(content.head).toEqual([])

    const broken = task("2026-11-13-broken", "切れた親", {
      frontmatter: fm({ parent: "gone.md" }),
      links: [link("parent", `${P}/tasks/gone.md`, "gone.md")],
    })
    expect(inspect(broken, linkIndex([broken])).family).toEqual([])
  })

  it("関連ファイルは置き場の名前の順に分け、中はキーの順に このファイルの値 → 逆向き。同じファイルは 1 回", () => {
    const here = task("2026-11-09-printable-menu", "印刷")
    const layout = doc(`${P}/artifacts/print-layout.md`, { title: "印刷のレイアウト", links: [link("tasks", here.path)] })
    const review = doc(`${P}/artifacts/print-review.md`, {
      title: "印刷のレビュー",
      links: [link("tasks", here.path), link("body", here.path)],
    })
    const note = doc(`${P}/inbox/2026-11-10-slow.md`, { title: "遅い気がする", links: [link("about", here.path)] })
    const material = doc(`${P}/materials/2026-11-10-patched.md`, {
      kind: "material",
      title: "個別に直した",
      links: [link("source", here.path)],
    })
    const knowledge = doc("knowledge/fix-structure.md", {
      title: "構造で直す",
      links: [link("sources", material.path), link("sources", here.path)],
    })
    const projectKnowledge = doc(`${P}/knowledge/print.md`, { title: "印刷の決まり", links: [link("sources", here.path)] })
    const content = inspect(here, linkIndex([here, layout, review, note, material, knowledge, projectKnowledge]))
    expect(content.related.map((group) => [group.label, titles(group.files)])).toEqual([
      ["artifacts", ["印刷のレイアウト", "印刷のレビュー"]],
      ["inbox", ["遅い気がする"]],
      // ルートとプロジェクトの knowledge は分けない。同じプロジェクト → ルートの順
      ["knowledge", ["印刷の決まり", "構造で直す"]],
      ["materials", ["個別に直した"]],
    ])
    // 本文のリンクは関連ファイルとは別に、他からのリンクに出る
    expect(titles(content.backlinks)).toEqual(["印刷のレビュー"])
  })

  it("素材の source と knowledge は関連ファイルに出し、library の source は頭に出す", () => {
    const here = doc(`${P}/materials/2026-11-10-patched.md`, {
      kind: "material",
      frontmatter: fm({ source: "../tasks/2026-11-09-printable-menu.md", knowledge: ["../../../knowledge/fix.md"] }),
      links: [
        link("source", `${P}/tasks/2026-11-09-printable-menu.md`, "../tasks/2026-11-09-printable-menu.md"),
        link("knowledge", "knowledge/fix.md", "../../../knowledge/fix.md"),
      ],
    })
    const source = task("2026-11-09-printable-menu", "印刷")
    const knowledge = doc("knowledge/fix.md", { title: "構造で直す" })
    const content = inspect(here, linkIndex([here, source, knowledge]))
    expect(content.head).toEqual([])
    expect(content.related.map((group) => [group.label, titles(group.files)])).toEqual([
      ["knowledge", ["構造で直す"]],
      ["tasks", ["印刷"]],
    ])
  })

  it("リンクとして読むキーでも、相対リンクにならなかった値（URL など）は頭にキーの綴りで出す。切れた相対リンクは出さない", () => {
    const pr = "https://github.com/example/recipe-app/pull/12"
    const knowledge = doc("knowledge/fix.md", { title: "構造で直す" })
    const here = doc(`${P}/materials/2026-11-10-patched.md`, {
      kind: "material",
      frontmatter: fm({ source: pr, knowledge: ["../../../knowledge/fix.md", "~/notes/fix.md", "../../../knowledge/gone.md"] }),
      links: [
        { ...link("source", pr), form: "external", target: "" },
        link("knowledge", "knowledge/fix.md", "../../../knowledge/fix.md"),
        { ...link("knowledge", "/Users/me/notes/fix.md", "~/notes/fix.md"), form: "home" },
        link("knowledge", "knowledge/gone.md", "../../../knowledge/gone.md"),
      ],
    })
    const content = inspect(here, linkIndex([here, knowledge]))
    expect(content.head).toEqual([
      { label: "source", value: pr, form: "url" },
      { label: "knowledge", value: "~/notes/fix.md", form: "text" },
    ])
    expect(content.related.map((group) => [group.label, titles(group.files)])).toEqual([["knowledge", ["構造で直す"]]])

    const task = doc(`${P}/tasks/2026-11-09-printable-menu.md`, {
      kind: "task",
      frontmatter: fm({ parent: "https://example.com/epic/1" }),
      links: [{ ...link("parent", "https://example.com/epic/1"), form: "external", target: "" }],
    })
    expect(inspect(task, linkIndex([task])).head).toEqual([
      { label: "parent", value: "https://example.com/epic/1", form: "url" },
    ])
  })

  it("逆向きの行は、左の列と同じ並び（日付で始まる名前は新しい順で先に、ほかは見出しの順）", () => {
    const here = task("2026-11-09-printable-menu", "印刷")
    const older = task("2026-11-10-decide-paper-size", "用紙", { links: [link("parent", here.path)] })
    const newer = task("2026-11-12-print-preview", "プレビュー", { links: [link("parent", here.path)] })
    const zeta = doc(`${P}/artifacts/zeta.md`, { title: "印刷のレビュー", links: [link("body", here.path)] })
    const alpha = doc(`${P}/artifacts/alpha.md`, { title: "印刷のレイアウト", links: [link("body", here.path)] })
    const note = doc(`${P}/inbox/2026-11-11-note.md`, { title: "メモ", links: [link("body", here.path)] })
    const content = inspect(here, linkIndex([here, older, newer, zeta, alpha, note]))
    expect(content.family.map((group) => [group.label, titles(group.files)])).toEqual([["子タスク", ["プレビュー", "用紙"]]])
    expect(titles(content.backlinks)).toEqual(["メモ", "印刷のレイアウト", "印刷のレビュー"])
  })

  it("リンクは場所の順（同じプロジェクト → ルート → ほかのプロジェクト → archive）で、場所の分からないものに場所を添える", () => {
    const here = doc(`${P}/wiki/design.md`, {
      title: "設計",
      links: [
        link("body", "projects/garden-log/wiki/water.md"),
        link("body", `${P}/archive/tasks/2026-10-01-old.md`),
        link("body", "principles/structure.md"),
        link("body", `${P}/wiki/api.md`),
        link("body", "archive/projects/old-app/wiki/x.md"),
        link("body", `${P}/wiki/design.md`),
        link("body", `${P}/wiki/api.md`),
        { ...link("body", "https://example.com"), form: "external" },
      ],
    })
    const docs = [
      here,
      doc("projects/garden-log/wiki/water.md", { title: "水やり" }),
      doc(`${P}/archive/tasks/2026-10-01-old.md`, { kind: "task", title: "前のタスク", status: "done" }),
      doc("principles/structure.md", { title: "構造で直す" }),
      doc(`${P}/wiki/api.md`, { title: "API" }),
      doc("archive/projects/old-app/wiki/x.md", { title: "前のアプリ" }),
    ]
    const content = inspect(here, linkIndex(docs))
    expect(titles(content.links)).toEqual([
      "API",
      "構造で直す",
      "水やり（garden-log）",
      "前のタスク（archive）",
      "前のアプリ（old-app の archive）",
    ])
    expect(content.links[3]?.status).toBe("done")
  })

  it("frontmatter の無いファイルでも、パスと他からのリンクは出す", () => {
    const image = doc(`${P}/artifacts/assets/print.png`, { is_md: false })
    const layout = doc(`${P}/artifacts/print-layout.md`, { title: "印刷のレイアウト", links: [link("body", image.path)] })
    const content = inspect(image, linkIndex([image, layout]))
    expect(content).toMatchObject({ path: image.path, head: [], code: [], family: [], related: [], links: [] })
    expect(titles(content.backlinks)).toEqual(["印刷のレイアウト"])
  })
})
