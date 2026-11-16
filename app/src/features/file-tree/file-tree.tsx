import {
  Tree,
  TreeGroup,
  TreeItem,
  TreeItemIndicator,
  TreeItemLabel,
  treeFromPaths,
  type TreeItemData,
} from "@ui-sdk/solid/tree"
import { createEffect, createMemo, For, Show } from "solid-js"
import { tree } from "../../../styled-system/recipes"
import { ChevronRight } from "../../components/icons"
import { HumanMark, StatusMark, type Status } from "../../components/status-mark"

export type FileEntry = {
  path: string
  title?: string
  status?: Status
  needsHuman?: boolean
}

type FileTreeProps = {
  files: readonly FileEntry[]
  /** 並べ方。省くとパスのフォルダーで並べる。値はパスで、files にあるものだけが選べる */
  items?: TreeItemData[]
  defaultExpanded?: string[]
  /** 今のタブのファイル。選んだ見た目になり、変わるたびに祖先を開いて、その行が見える所まで送る */
  selected: string | null
  /** ファイルの行を押したとき */
  onSelect: (path: string) => void
}

type TreeClasses = ReturnType<typeof tree>

export function FileTree(props: FileTreeProps) {
  const classes = tree()
  const items = createMemo(() => props.items ?? treeFromPaths(props.files.map((file) => file.path)))
  const byPath = createMemo(() => new Map(props.files.map((file) => [file.path, file])))
  const selected = () => (props.selected !== null && byPath().has(props.selected) ? props.selected : null)
  let root: HTMLDivElement | undefined

  // 今のファイルが変わったら、その行が見える所まで送る。祖先は Tree が current で開く。
  // UI SDK の controller の reveal はフォーカスも動かすので使わない（タブを切り替えたとき、フォーカスはタブに残す）
  createEffect(selected, (path) => {
    if (path === null) return
    const frame = requestAnimationFrame(() => {
      const row = root?.querySelector<HTMLElement>(`[data-slot=tree-label][data-path="${CSS.escape(path)}"]`)
      row?.scrollIntoView?.({ block: "nearest", inline: "nearest" })
    })
    return () => cancelAnimationFrame(frame)
  })

  return (
    <Tree
      ref={(el: HTMLDivElement) => (root = el)}
      aria-label="ファイル"
      class={classes.root}
      items={items()}
      defaultExpanded={props.defaultExpanded}
      current={selected()}
      value={selected()}
      onValueChange={(value, details) => {
        // 選べるのはファイルだけ。フォルダーは押すと開閉し、選択は前のファイルのまま残す。
        // 選んでいる行で Space を押すか ⌘ を押しながら押すと、Tree は選択を外そうとする（null）。何も開いていない状態を作らないよう取り消す
        if (typeof value !== "string" || !byPath().has(value)) {
          details.cancel()
          return
        }
        props.onSelect(value)
      }}
    >
      <Items items={items()} byPath={byPath()} classes={classes} />
    </Tree>
  )
}

function Items(props: {
  items: TreeItemData[]
  byPath: Map<string, FileEntry>
  classes: TreeClasses
}) {
  // 行は値（パス）で見分ける。索引をもらい直しても、同じパスの行は作り直さない（フォーカスを保つ）
  return (
    <For each={props.items} keyed={(item) => item.value}>
      {(item) => {
        const file = () => props.byPath.get(item().value)
        return (
          <TreeItem value={item().value} class={props.classes.item}>
            <TreeItemLabel class={props.classes.label} data-path={file() ? item().value : undefined}>
              {/* 行の頭の枠は 1 つ。フォルダーは開閉の矢印、タスクは status の印。印の無いファイルは枠を持たない */}
              <Show
                when={item().children}
                fallback={
                  <Show when={file()?.status}>
                    {(status) => (
                      <span class={props.classes.lead}>
                        <StatusMark status={status()} />
                      </span>
                    )}
                  </Show>
                }
              >
                <span class={props.classes.lead}>
                  <TreeItemIndicator class={props.classes.indicator}>
                    <ChevronRight />
                  </TreeItemIndicator>
                </span>
              </Show>
              <span class={props.classes.text}>{file()?.title ?? item().label}</span>
              <Show when={file()?.needsHuman}>
                <span class={props.classes.trail}>
                  <HumanMark />
                </span>
              </Show>
            </TreeItemLabel>
            <Show when={item().children}>
              {(children) => (
                <TreeGroup class={props.classes.group}>
                  <Items items={children()} byPath={props.byPath} classes={props.classes} />
                </TreeGroup>
              )}
            </Show>
          </TreeItem>
        )
      }}
    </For>
  )
}
