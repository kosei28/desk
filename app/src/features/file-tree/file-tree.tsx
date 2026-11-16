import {
  Tree,
  TreeGroup,
  TreeItem,
  TreeItemIndicator,
  TreeItemLabel,
  treeFromPaths,
  type TreeItemData,
} from "@ui-sdk/solid/tree"
import { createMemo, For, Show } from "solid-js"
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
  selected: string | null
  onSelect: (path: string) => void
}

type TreeClasses = ReturnType<typeof tree>

export function FileTree(props: FileTreeProps) {
  const classes = tree()
  const items = createMemo(() => props.items ?? treeFromPaths(props.files.map((file) => file.path)))
  const byPath = createMemo(() => new Map(props.files.map((file) => [file.path, file])))

  return (
    <Tree
      aria-label="ファイル"
      class={classes.root}
      items={items()}
      defaultExpanded={props.defaultExpanded}
      value={props.selected !== null && byPath().has(props.selected) ? props.selected : null}
      onValueChange={(value, details) => {
        // 選べるのはファイルだけ。フォルダーは押すと開閉し、選択は前のファイルのまま残す
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
  return (
    <For each={props.items}>
      {(item) => {
        const file = () => props.byPath.get(item.value)
        return (
          <TreeItem value={item.value} class={props.classes.item}>
            <TreeItemLabel class={props.classes.label} data-path={file() ? item.value : undefined}>
              {/* 行の頭の枠は 1 つ。フォルダーは開閉の矢印、タスクは status の印。印の無いファイルは枠を持たない */}
              <Show
                when={item.children}
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
              <span class={props.classes.text}>{file()?.title ?? item.label}</span>
              <Show when={file()?.needsHuman}>
                <span class={props.classes.trail}>
                  <HumanMark />
                </span>
              </Show>
            </TreeItemLabel>
            <Show when={item.children}>
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
