import type { JSX } from "@solidjs/web"
import {
  createMenuController,
  Menu,
  MenuContent,
  MenuContextTrigger,
  MenuItem,
  MenuSeparator,
  type MenuController,
} from "@ui-sdk/solid/menu"
import { createSignal, For, Show } from "solid-js"
import { menu } from "../../styled-system/recipes"

export type PathAction = "open-in-new-tab" | "close-tab" | "copy-path" | "copy-relative-path"

// キーの組み合わせはまだ出さない。キーをつなぐまで、効かないキーをメニューに見せないため
const labels: Record<PathAction, string> = {
  "open-in-new-tab": "新しいタブで開く",
  "close-tab": "タブを閉じる",
  "copy-path": "パスをコピー",
  "copy-relative-path": "ルートからのパスをコピー",
}

// 開く・閉じると、コピーを別のまとまりにして線で区切る
const opens = (action: PathAction) => action === "open-in-new-tab" || action === "close-tab"

type PathMenuProps = {
  /** メニューに出す項目。並びはこの順で、開く・閉じると、コピーの間に線を引く */
  actions: readonly PathAction[]
  onAction: (action: PathAction, path: string) => void
  /** 外から開くとき（キー・テスト・スクショ）に使う。payload はパス */
  controller?: MenuController<string>
  class?: string
  children: JSX.Element
}

// 右クリックのメニュー。中の要素のうち data-path を持つもの（左の列の行・タブ・右の列のリンク・読む面の見出し）を
// 右クリックすると、そのパスについてのメニューを開く。開いている間、その要素に data-menu-open を付けて、どれのメニューかを示す
export function PathMenu(props: PathMenuProps) {
  const controller = props.controller ?? createMenuController<string>()
  const classes = menu()
  const [target, setTarget] = createSignal<string | null>(null)
  let area: HTMLDivElement | undefined
  let marked: Element | undefined

  const mark = (path: string | null) => {
    marked?.removeAttribute("data-menu-open")
    marked = path === null ? undefined : (area?.querySelector(`[data-path="${CSS.escape(path)}"]`) ?? undefined)
    marked?.setAttribute("data-menu-open", "")
  }
  const pathAt = (node: EventTarget | null) =>
    node instanceof Element ? (node.closest<HTMLElement>("[data-path]")?.dataset.path ?? null) : null

  const groups = () => {
    const first = props.actions.filter(opens)
    const rest = props.actions.filter((action) => !opens(action))
    return [first, rest].filter((group) => group.length > 0)
  }

  return (
    <Menu
      controller={controller}
      onOpenChange={(open, details) => {
        if (!open) {
          mark(null)
          return
        }
        // 外から（controller で）開くときは、パスは payload で届く（onPayloadChange）
        if (details.reason === "imperative") return
        // キー（Shift+F10）で開いたときは、フォーカスのある行のパスを使う
        const path = target() ?? pathAt(document.activeElement)
        if (path === null) {
          details.cancel()
          return
        }
        setTarget(path)
        mark(path)
      }}
      onPayloadChange={(payload) => {
        if (typeof payload !== "string") return
        setTarget(payload)
        mark(payload)
      }}
      onAction={(value) => {
        const path = target()
        if (path !== null) props.onAction(value as PathAction, path)
      }}
    >
      <MenuContextTrigger
        ref={area}
        class={props.class}
        onContextMenu={(event: MouseEvent) => setTarget(pathAt(event.target))}
      >
        {props.children}
      </MenuContextTrigger>
      <MenuContent class={classes.content}>
        <For each={groups()}>
          {(group, i) => (
            <>
              <Show when={i() > 0}>
                <MenuSeparator class={classes.separator} />
              </Show>
              <For each={group}>
                {(action) => (
                  <MenuItem class={classes.item} value={action}>
                    {labels[action]}
                  </MenuItem>
                )}
              </For>
            </>
          )}
        </For>
      </MenuContent>
    </Menu>
  )
}
