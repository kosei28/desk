import { ScrollArea, ScrollAreaContent, ScrollAreaViewport } from "@ui-sdk/solid/scroll-area"
import { TabsList, TabsTab } from "@ui-sdk/solid/tabs"
import { For, Show } from "solid-js"
import { cx } from "../../styled-system/css"
import { iconButton, tabs } from "../../styled-system/recipes"
import { Cross } from "./icons"
import { StatusMark, type Status } from "./status-mark"

export type TabEntry = {
  path: string
  title: string
  status?: Status
}

type FileTabListProps = {
  tabs: readonly TabEntry[]
  onClose: (path: string) => void
  class?: string
}

// UI SDK の Tabs の中に置く、開いているファイルのタブの並び。タブの値はパス。
// 閉じるボタンはタブ（button）の中に入れられないので隣に置き、タブ順には入れない。
// キーで閉じるときは、タブにフォーカスを置いて Shift+F10 で右クリックのメニュー（PathMenu）を開き、「タブを閉じる」を選ぶ。
// 並びは UI SDK の ScrollArea の中に置き、送れる側の端をぼかす（見た目はレシピ）
export function FileTabList(props: FileTabListProps) {
  const classes = tabs()
  return (
    <ScrollArea aria-label="タブの並び" class={cx(classes.area, props.class)}>
      <ScrollAreaViewport class={classes.viewport}>
        <ScrollAreaContent>
    <TabsList aria-label="開いているファイル" class={classes.list}>
      {/* タブはパスで見分ける。見出しや印が変わっても、同じパスのタブは作り直さない（フォーカスと退場中のパネルを保つ） */}
      <For each={props.tabs} keyed={(tab) => tab.path}>
        {(tab) => (
          <div class={classes.item} data-path={tab().path}>
            <TabsTab value={tab().path} class={classes.tab}>
              <Show when={tab().status}>{(status) => <StatusMark status={status()} />}</Show>
              <span class={classes.title}>{tab().title}</span>
            </TabsTab>
            <button
              type="button"
              tabindex={-1}
              aria-label={`${tab().title} を閉じる`}
              class={cx(classes.close, iconButton())}
              onClick={() => props.onClose(tab().path)}
            >
              <Cross />
            </button>
          </div>
        )}
      </For>
    </TabsList>
        </ScrollAreaContent>
      </ScrollAreaViewport>
    </ScrollArea>
  )
}
