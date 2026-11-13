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
import { HumanMark, StatusMark, type Status } from "../../components/status-mark"

export type FileEntry = {
  path: string
  title?: string
  status?: Status
  needsHuman?: boolean
}

type FileTreeProps = {
  files: readonly FileEntry[]
  selected: string | null
  onSelect: (path: string) => void
}

type TreeClasses = ReturnType<typeof tree>

export function FileTree(props: FileTreeProps) {
  const classes = tree()
  const items = createMemo(() => treeFromPaths(props.files.map((file) => file.path)))
  const byPath = createMemo(() => new Map(props.files.map((file) => [file.path, file])))

  return (
    <Tree
      aria-label="ファイル"
      class={classes.root}
      items={items()}
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
            <TreeItemLabel class={props.classes.label}>
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

function ChevronRight() {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      stroke-width="1.75"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <path d="m6.5 4.5 3.5 3.5-3.5 3.5" />
    </svg>
  )
}
