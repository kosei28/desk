import type { Status } from "../../components/status-mark"
import type { Doc, FrontmatterValue } from "../../data/types"
import { isUrl } from "../../data/url-schemes"
import { compareFiles, fileEntry, statuses } from "../file-tree/desk-tree"
import type { FileEntry } from "../file-tree/file-tree"

// 右の列の中身を、索引から求める（[情報設計の決まり] の「右の列」）。形を決めるのはここで、部品は並べるだけ。
// リンクの行き先は core の答え（Doc の links の target）を使い、TS では解かない

/** 右の列のファイルの行。左の列の行と同じものに、場所の添え書きを足す */
export type InspectorFile = FileEntry & {
  /** 場所が行から分からないときに添える場所（別のプロジェクトの名前・archive） */
  place?: string
}

/** 頭とコードの行 1 つ */
export type InspectorValue = {
  label: string
  value: string
  /** url は外の URL 1 つ（scheme は本文と同じ urlSchemes。押すと既定のアプリで開く）。path はパスとブランチ名（repo・branch・ベースブランチ・worktree。等幅） */
  form: "text" | "url" | "path"
}

/** 名前の下にファイルの行を並べたもの（親子と依存・関連ファイル） */
export type InspectorGroup = {
  label: string
  /** 空なら「[]」と出す（親子と依存で、空のリストを書いてあるとき） */
  files: InspectorFile[]
}

export type InspectorContent = {
  path: string
  /** 頭（文字の値）。書いてある順 */
  head: InspectorValue[]
  status?: Status
  needsHuman: boolean
  tags: string[]
  code: InspectorValue[]
  /** 親子と依存 */
  family: InspectorGroup[]
  /** 関連ファイル。置き場（ディレクトリ名）の名前の順 */
  related: InspectorGroup[]
  /** リンク（このファイルの本文が指すファイル） */
  links: InspectorFile[]
  /** 他からのリンク（ほかのファイルの本文が、このファイルを指しているもの） */
  backlinks: InspectorFile[]
}

/** あるファイルを指すリンク 1 つ（逆向きを引くため） */
type Incoming = { from: Doc; key: string }

/** 索引から一度だけ作る、パスの引き当てと逆向きのリンク */
export type LinkIndex = {
  byPath: ReadonlyMap<string, Doc>
  /** 指されているファイルのパス → それを指すリンク（索引の順） */
  incoming: ReadonlyMap<string, readonly Incoming[]>
}

export function linkIndex(docs: readonly Doc[]): LinkIndex {
  const byPath = new Map(docs.map((doc) => [doc.path, doc]))
  const incoming = new Map<string, Incoming[]>()
  for (const from of docs) {
    for (const link of from.links) {
      if (link.form !== "relative" || link.target === from.path || !byPath.has(link.target)) continue
      const list = incoming.get(link.target) ?? []
      list.push({ from, key: link.key })
      incoming.set(link.target, list)
    }
  }
  return { byPath, incoming }
}

/** 頭に名前を付けて出すキー。ほかの、行き場の決まっていないキーは綴りのまま出す */
const headLabels: Record<string, string> = {
  description: "概要",
  url: "URL",
  source: "出どころ",
  verified: "確認日",
}

/** コードに出すキー。この順に並べる。ベースブランチは branch の行のコメントから */
const codeKeys = ["repo", "branch", "base", "worktree", "pr"] as const
const codeLabels: Record<(typeof codeKeys)[number], string> = {
  repo: "リポジトリ",
  branch: "ブランチ",
  base: "ベースブランチ",
  worktree: "ワークツリー",
  pr: "PR",
}

/** 親子と依存のキー。キーごとに このファイルの値 → 逆向き */
const familyKeys = [
  { key: "parent", own: "親タスク", reverse: "子タスク" },
  { key: "depends_on", own: "先行タスク", reverse: "後続タスク" },
] as const

/** 関連ファイルのキー。置き場の中は、この順に、キーごとに このファイルの値 → 逆向き */
const relatedKeys = ["tasks", "about", "source", "principle", "sources", "knowledge"] as const

const collator = new Intl.Collator("ja", { numeric: true })

const textOf = (value: FrontmatterValue): string =>
  value === null ? "" : typeof value === "string" ? value : value.join(", ")

/** コードのうち、パスとブランチ名として出すキー */
const pathKeys: readonly string[] = ["repo", "branch", "base", "worktree"]

const formOf = (key: string, value: string): InspectorValue["form"] =>
  pathKeys.includes(key) ? "path" : isUrl(value) ? "url" : "text"

/**
 * 置き場（関連ファイルの分け）。プロジェクトとルート、archive を外した最初のディレクトリ名（wiki の入れ子や assets の中も、上の置き場）。
 * プロジェクトの直下のもの（README）は、そのディレクトリの名前
 */
function placeDir(path: string): string {
  const parts = path.split("/")
  if (parts[0] === "archive") parts.shift()
  if (parts[0] === "projects") {
    if (parts.length === 3) return parts[1]
    parts.splice(0, 2)
  }
  if (parts[0] === "archive") parts.shift()
  return parts.length > 1 ? parts[0] : "."
}

/** 場所の順（同じプロジェクト → ルート → ほかのプロジェクト → archive の中）と、添え書き */
function placeOf(doc: Doc, here: Doc): { rank: number; label?: string } {
  const archived = doc.archived || doc.project_archived
  const other = doc.project !== "" && doc.project !== here.project
  const label = other ? (archived ? `${doc.project} の archive` : doc.project) : archived ? "archive" : undefined
  const rank = archived ? 3 : doc.project === here.project ? 0 : doc.project === "" ? 1 : 2
  return { rank, label }
}

/** ファイルを場所の順に並べて行にする。同じ場所の中は、渡した順のまま。同じファイルは 1 回 */
function rows(docs: readonly Doc[], here: Doc): InspectorFile[] {
  const seen = new Set<string>()
  const unique = docs.filter((doc) => doc.path !== here.path && !seen.has(doc.path) && seen.add(doc.path))
  return unique
    .map((doc) => ({ doc, place: placeOf(doc, here) }))
    .sort((a, b) => a.place.rank - b.place.rank)
    .map(({ doc, place }) => ({ ...fileEntry(doc), place: place.label }))
}

/** 今のファイルの右の列の中身 */
export function inspect(doc: Doc, index: LinkIndex): InspectorContent {
  const fields = doc.frontmatter ?? []
  const linkKeys = new Set(doc.links.filter((link) => link.key !== "body").map((link) => link.key))
  const isTask = doc.kind === "task"
  const incoming = index.incoming.get(doc.path) ?? []

  /** このファイルが key で指すファイル（書いてある順） */
  const own = (key: string) =>
    doc.links.flatMap((link) => {
      if (link.key !== key || link.form !== "relative") return []
      const target = index.byPath.get(link.target)
      return target ? [target] : []
    })
  /** key でこのファイルを指しているファイル。書いてある順が無いので、左の列と同じ並び */
  const reverse = (key: string) =>
    incoming
      .filter((link) => link.key === key)
      .map((link) => link.from)
      .sort(compareFiles)
  /** key の値のうち、相対リンクにならなかったもの（URL・~ 始まりなど）。切れた相対リンクは入れない */
  const notLinked = (key: string, value: FrontmatterValue) => {
    const relative = new Set(
      doc.links.filter((link) => link.key === key && link.form === "relative").map((link) => link.raw),
    )
    const values = value === null ? [] : typeof value === "string" ? [value] : value
    return values.filter((v) => v.trim() !== "" && !relative.has(v))
  }

  // いちばん上（頭と札）とコード。リンクとして読むキーは見出しのほうに出し、相対リンクにならなかった値だけを頭に綴りのキーで出す
  const head: InspectorValue[] = []
  const tags: string[] = []
  const code = new Map<string, string>()
  let status: Status | undefined
  let needsHuman = false
  for (const field of fields) {
    const { key, value } = field
    if (linkKeys.has(key) || (isTask && familyKeys.some((f) => f.key === key))) {
      const rest = notLinked(key, value).join(", ")
      if (rest) head.push({ label: key, value: rest, form: isUrl(rest) ? "url" : "text" })
      continue
    }
    const text = textOf(value)
    if (key === "tags") {
      tags.push(...(Array.isArray(value) ? value : value ? [value] : []))
      continue
    }
    if (key === "status" && statuses.includes(text)) {
      status ??= text as Status
      continue
    }
    if (key === "needs_human" && (text === "true" || text === "false")) {
      needsHuman ||= text === "true"
      continue
    }
    if ((codeKeys as readonly string[]).includes(key) && key !== "base") {
      if (text && !code.has(key)) code.set(key, text)
      continue
    }
    if (!text) continue
    head.push({ label: headLabels[key] ?? key, value: text, form: isUrl(text) ? "url" : "text" })
  }
  if (doc.base && code.has("branch")) code.set("base", doc.base)

  // 親子と依存。空のリストを書いてあれば「[]」と出す
  const family: InspectorGroup[] = []
  for (const { key, own: ownLabel, reverse: reverseLabel } of familyKeys) {
    const written = fields.find((field) => field.key === key)?.value
    const mine = rows(own(key), doc)
    if (mine.length > 0 || (Array.isArray(written) && written.length === 0)) family.push({ label: ownLabel, files: mine })
    const theirs = rows(reverse(key), doc)
    if (theirs.length > 0) family.push({ label: reverseLabel, files: theirs })
  }

  // 関連ファイル。置き場で分け、同じファイルは 1 回
  const linked = relatedKeys.flatMap((key) => [...rows(own(key), doc), ...rows(reverse(key), doc)])
  const seen = new Set<string>()
  const byDir = new Map<string, InspectorFile[]>()
  for (const file of linked) {
    if (seen.has(file.path)) continue
    seen.add(file.path)
    const dir = placeDir(file.path)
    byDir.set(dir, [...(byDir.get(dir) ?? []), file])
  }
  const related = [...byDir]
    .sort(([a], [b]) => collator.compare(a, b))
    .map(([label, files]) => ({ label, files }))

  return {
    path: doc.path,
    head,
    status,
    needsHuman,
    tags,
    code: codeKeys.flatMap((key) => {
      const value = code.get(key)
      return value ? [{ label: codeLabels[key], value, form: formOf(key, value) }] : []
    }),
    family,
    related,
    links: rows(own("body"), doc),
    backlinks: rows(reverse("body"), doc),
  }
}
