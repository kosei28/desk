import { Resizable, ResizableHandle, ResizablePanel } from "@ui-sdk/solid/resizable"
import { useShortcut } from "@ui-sdk/solid/shortcuts"
import { Tabs, TabsPanel } from "@ui-sdk/solid/tabs"
import { createEffect, createMemo, For, Show } from "solid-js"
import { css, cx } from "../styled-system/css"
import { link, resizeHandle } from "../styled-system/recipes"
import { FileTabList, type TabEntry } from "./components/file-tabs"
import { Surface } from "./components/surface"
import { createDesk } from "./data/desk"
import type { Doc } from "./data/types"
import { createColumns } from "./features/columns/columns"
import { deskTreeItems, fileEntry } from "./features/file-tree/desk-tree"
import { FileTree } from "./features/file-tree/file-tree"
import { dragRegion } from "./features/window/drag-region"

const baseName = (path: string) => path.slice(path.lastIndexOf("/") + 1)

/** タブと島の見出しの言葉。見出しが無ければファイル名 */
const titleOf = (doc: Doc) => doc.title || baseName(doc.path)

// 帯（窓を動かす所）の上に重ねる入れ物は押せないようにし、中の押せるもの（行・タブ・島・列の境目）だけを押せるようにする。
// 帯の高さの中で何にも覆われていない所（左の列の上・タブの並びの空いた所・右の列の上）は、どこも帯に当たる
const passThrough = css({ pointerEvents: "none" })
const pressable = css({ pointerEvents: "auto" })

export function App() {
  const desk = createDesk()
  const columns = createColumns()

  const files = createMemo(() => desk.index()?.docs.map(fileEntry) ?? [])
  const items = createMemo(() => deskTreeItems(desk.index()?.docs ?? []))
  const byPath = createMemo(() => new Map(desk.index()?.docs.map((doc) => [doc.path, doc])))
  const tabs = createMemo(() =>
    desk.tabs().flatMap((path): TabEntry[] => {
      const doc = byPath().get(path)
      if (!doc) return []
      return [{ path, title: titleOf(doc), status: fileEntry(doc).status }]
    }),
  )

  // ⌘W はタブが無いときも受け取り、窓は閉じない（メニューの「ウインドウを閉じる」に渡さない）
  useShortcut("mod+w", () => {
    const path = desk.selected()
    if (path !== null) desk.close(path)
  })
  useShortcut("ctrl+tab", () => desk.cycle(1), { when: () => desk.tabs().length > 1 })
  useShortcut("ctrl+shift+tab", () => desk.cycle(-1), { when: () => desk.tabs().length > 1 })

  // 今のタブが変わったら、タブの並びをそのタブが見える所まで送る（送れる側の端のぼかしの分は、レシピの scroll-padding で空ける）
  let tabStrip: HTMLDivElement | undefined
  createEffect(desk.selected, (path) => {
    if (path === null) return
    const frame = requestAnimationFrame(() => {
      const tab = tabStrip?.querySelector<HTMLElement>(`[data-path="${CSS.escape(path)}"]`)
      tab?.scrollIntoView?.({ block: "nearest", inline: "nearest" })
    })
    return () => cancelAnimationFrame(frame)
  })

  return (
    <div class={css({ position: "relative", height: "full" })}>
      {/* 帯はウィンドウの幅いっぱいの層。列とタブと島はその上に載り、覆われていない所で窓を動かす */}
      <div
        ref={dragRegion}
        data-titlebar
        class={css({ position: "absolute", insetInline: "0", top: "0", height: "titlebar", zIndex: "titlebar" })}
      />
      <Resizable
        ref={columns.ref}
        aria-label="列"
        layout={columns.layout()}
        onLayoutChange={columns.onLayoutChange}
        class={cx(passThrough, css({ position: "absolute", inset: "0", display: "flex" }))}
      >
        <ResizablePanel
          id="left"
          defaultSize={columns.left.defaultSize}
          minSize={columns.left.minSize}
          maxSize={columns.left.maxSize}
          class={cx(passThrough, css({ display: "flex", flexDirection: "column", paddingTop: "12" }))}
        >
          <nav
            aria-label="ツリー"
            class={cx(
              pressable,
              css({ flex: "1", minHeight: "0", overflow: "auto", paddingInlineStart: "2", paddingBottom: "2" }),
            )}
          >
            <Show when={desk.index()?.root} keyed>
              {/* 別のツリーに移ったら作り直し、開いていたフォルダーを忘れる。同じツリーで索引をもらい直しても作り直さない */}
              <FileTree
                files={files()}
                items={items()}
                selected={desk.selected()}
                onSelect={(path) => desk.open(path)}
              />
            </Show>
          </nav>
        </ResizablePanel>
        <ResizableHandle
          aria-label="左の列の幅"
          class={cx(resizeHandle(), pressable, css({ marginTop: "12" }))}
          onDblClick={() => columns.reset("left")}
        />
        <ResizablePanel
          id="main"
          minSize={columns.main.minSize}
          class={cx(passThrough, css({ display: "flex", flexDirection: "column", minWidth: "0", paddingBottom: "2" }))}
        >
          <Tabs
            value={desk.selected()}
            onValueChange={(path) => path !== null && desk.open(path)}
            class={css({ display: "contents" })}
          >
            {/* タブは上から 12〜40px（信号機と縦の中央をそろえる） */}
            <div
              ref={tabStrip}
              class={cx(
                passThrough,
                css({
                  display: "flex",
                  height: "[token(spacing.12)]",
                  paddingTop: "3",
                  paddingBottom: "2",
                  flexShrink: "0",
                  // 押せるのはタブだけ。並びの空いた所（最後のタブの右）は帯に当たる
                  "& [data-path]": { pointerEvents: "auto" },
                }),
              )}
            >
              <FileTabList tabs={tabs()} onClose={(path) => desk.close(path)} class={css({ flex: "1" })} />
            </div>
            <Surface
              class={cx(
                pressable,
                css({ flex: "1", minHeight: "0", zIndex: "surface" }),
                !desk.selectedDoc() && css({ alignContent: "center" }),
              )}
            >
              <Show when={desk.loaded()}>
                <Show when={desk.index()} fallback={<NoTree error={desk.error()} onChoose={() => void desk.choose()} />}>
                  <Show
                    when={desk.selectedDoc()}
                    fallback={<p class={css({ textAlign: "center", color: "fg.muted" })}>ファイルを選ぶと、ここに開く</p>}
                  >
                    {/* 本文・右の列・右クリックのメニューは順 5 */}
                    <For each={tabs()} keyed={(tab) => tab.path}>
                      {(tab) => (
                        <TabsPanel value={tab().path} class={css({ outline: "none" })}>
                          <header>
                            <h1 class={css({ textStyle: "title" })}>{tab().title}</h1>
                          </header>
                        </TabsPanel>
                      )}
                    </For>
                  </Show>
                </Show>
              </Show>
            </Surface>
          </Tabs>
        </ResizablePanel>
        <ResizableHandle
          aria-label="右の列の幅"
          class={cx(resizeHandle(), pressable, css({ marginTop: "12" }))}
          onDblClick={() => columns.reset("right")}
        />
        <ResizablePanel
          id="right"
          defaultSize={columns.right.defaultSize}
          minSize={columns.right.minSize}
          maxSize={columns.right.maxSize}
          class={passThrough}
        >
          {/* 右の列（Inspector）は順 5 で置く */}
        </ResizablePanel>
      </Resizable>
    </div>
  )
}

function NoTree(props: { error: string | null; onChoose: () => void }) {
  return (
    <div class={css({ display: "grid", justifyItems: "center", gap: "2", textAlign: "center" })}>
      <button type="button" class={link()} onClick={props.onChoose}>
        desk のツリーのフォルダーを選ぶ
      </button>
      <Show when={props.error}>{(error) => <p class={css({ color: "fg.muted" })}>{error()}</p>}</Show>
    </div>
  )
}
