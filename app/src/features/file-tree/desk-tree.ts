import type { TreeItemData } from "@ui-sdk/solid/tree"
import type { Status } from "../../components/status-mark"
import type { Doc } from "../../data/types"
import type { FileEntry } from "./file-tree"

export const statuses: readonly string[] = ["backlog", "todo", "inprogress", "done", "dropped"] satisfies Status[]

/** 日付で始まる名前（タスク・素材・inbox・library）の日付 */
const datePrefix = /^(\d{4}-\d{2}-\d{2})-/

/** どの段でも最後に、この順で置くフォルダー */
const lastFolders = ["archive", "assets"]

const collator = new Intl.Collator("ja", { numeric: true })

const baseName = (path: string) => path.slice(path.lastIndexOf("/") + 1)

/**
 * 左の列の行の言葉。README は「README」、`assets` の中はファイル名、ほかは見出し（無ければファイル名）
 */
export function rowLabel(doc: Doc): string {
  const name = baseName(doc.path)
  if (name === "README.md") return "README"
  if (doc.path.split("/").slice(0, -1).includes("assets")) return name
  return doc.title || name
}

/** 行に出すファイルのこと。status の印と人の印はタスクにだけ付ける */
export function fileEntry(doc: Doc): FileEntry {
  const task = doc.kind === "task"
  return {
    path: doc.path,
    title: rowLabel(doc),
    status: task && statuses.includes(doc.status) ? (doc.status as Status) : undefined,
    needsHuman: task && doc.needs_human,
  }
}

type Folder = { name: string; path: string; folders: Map<string, Folder>; files: Doc[] }

/**
 * 索引のファイルを、ディレクトリの構造そのままの木にする（[情報設計の決まり] の「左の列とタブ」）。
 * どの段も README → フォルダー（名前の順）→ ファイル（見出しの順。日付で始まる名前は新しい順で先に）→ `archive`・`assets`。
 * フォルダーの値はルートからのパス（/ で終わらない）、ファイルの値はルートからのパス
 */
export function deskTreeItems(docs: readonly Doc[]): TreeItemData[] {
  const root: Folder = { name: "", path: "", folders: new Map(), files: [] }
  for (const doc of docs) {
    const parts = doc.path.split("/")
    let folder = root
    for (const name of parts.slice(0, -1)) {
      let next = folder.folders.get(name)
      if (!next) {
        next = { name, path: folder.path ? `${folder.path}/${name}` : name, folders: new Map(), files: [] }
        folder.folders.set(name, next)
      }
      folder = next
    }
    folder.files.push(doc)
  }
  return itemsOf(root)
}

function itemsOf(folder: Folder): TreeItemData[] {
  const readme = folder.files.filter((doc) => baseName(doc.path) === "README.md")
  const files = folder.files.filter((doc) => baseName(doc.path) !== "README.md").sort(compareFiles)
  const folders = [...folder.folders.values()]
  const named = folders.filter((f) => !lastFolders.includes(f.name)).sort((a, b) => collator.compare(a.name, b.name))
  const last = lastFolders.flatMap((name) => folders.filter((f) => f.name === name))

  const fileItem = (doc: Doc): TreeItemData => ({ value: doc.path, label: rowLabel(doc) })
  const folderItem = (f: Folder): TreeItemData => ({ value: f.path, label: f.name, children: itemsOf(f) })
  return [...readme.map(fileItem), ...named.map(folderItem), ...files.map(fileItem), ...last.map(folderItem)]
}

/** 左の列の、同じ段のファイルの並び。日付で始まる名前は新しい順で先に、ほかは見出しの順 */
export function compareFiles(a: Doc, b: Doc): number {
  const dateA = datePrefix.exec(baseName(a.path))?.[1]
  const dateB = datePrefix.exec(baseName(b.path))?.[1]
  if (dateA && dateB && dateA !== dateB) return dateA < dateB ? 1 : -1
  if (dateA && !dateB) return -1
  if (!dateA && dateB) return 1
  return collator.compare(rowLabel(a), rowLabel(b)) || collator.compare(a.path, b.path)
}
