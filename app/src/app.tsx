import { Resizable, ResizableHandle, ResizablePanel } from "@ui-sdk/solid/resizable"
import { useShortcut } from "@ui-sdk/solid/shortcuts"
import { Tabs, TabsPanel } from "@ui-sdk/solid/tabs"
import { createEffect, createMemo, createSignal, For, Show, untrack } from "solid-js"
import { css, cx } from "../styled-system/css"
import { link, resizeHandle } from "../styled-system/recipes"
import { FileTabList, type TabEntry } from "./components/file-tabs"
import { Surface } from "./components/surface"
import { openUrl } from "./data/commands"
import { createDesk } from "./data/desk"
import type { Doc } from "./data/types"
import { createColumns } from "./features/columns/columns"
import { deskTreeItems, fileEntry } from "./features/file-tree/desk-tree"
import { FileTree } from "./features/file-tree/file-tree"
import { DeskInspector } from "./features/inspector/desk-inspector"
import { inspect, linkIndex } from "./features/inspector/inspect"
import { createSectionMemory } from "./features/inspector/sections"
import { FilePreview } from "./features/preview/file-preview"
import { dragRegion } from "./features/window/drag-region"

const baseName = (path: string) => path.slice(path.lastIndexOf("/") + 1)

/** 外の URL を既定のアプリで開く（右の列の URL の値と、本文のリンク） */
const openExternal = (url: string) => void openUrl(url).catch((e: unknown) => console.error(e))

/** タブと島の見出しの言葉。見出しが無ければファイル名 */
const titleOf = (doc: Doc) => doc.title || baseName(doc.path)

// 帯（窓を動かす所）の上に重ねる入れ物は押せないようにし、中の押せるもの（行・タブ・島・列の境目）だけを押せるようにする。
// 帯の高さの中で何にも覆われていない所（左の列の上・タブの並びの空いた所・右の列の上）は、どこも帯に当たる
const passThrough = css({ pointerEvents: "none" })
const pressable = css({ pointerEvents: "auto" })

export function App() {
  const desk = createDesk()
  const columns = createColumns()

  const sections = createSectionMemory()

  const files = createMemo(() => desk.index()?.docs.map(fileEntry) ?? [])
  const items = createMemo(() => deskTreeItems(desk.index()?.docs ?? []))
  const links = createMemo(() => linkIndex(desk.index()?.docs ?? []))
  const inspected = createMemo(() => {
    const doc = desk.selectedDoc()
    return doc ? inspect(doc, links()) : undefined
  })
  const tabs = createMemo(() =>
    desk.tabs().flatMap((path): TabEntry[] => {
      const doc = links().byPath.get(path)
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

  // 島のスクロールの位置はタブごとに覚え、タブを切り替えたらそのタブの位置に戻す。新しく開いたタブは上から。
  // 島は全部のタブで 1 つなので、覚えないと前のタブの位置のまま次のタブが出る
  const scrolls = new Map<string, number>()
  let island: HTMLElement | undefined
  const rememberScroll = () => {
    const path = desk.selected()
    if (path !== null && island) scrolls.set(path, island.scrollTop)
  }
  // 本文のリンク（file.md#見出し）で開いたときの送る先。そのタブが出て、中身を描けたら送り、消す。
  // 送るまでそのタブは隠し、島の位置も戻さない（上から描いたものや、前の位置の絵を見せない）
  const [anchor, setAnchor] = createSignal<{ path: string; id: string }>()
  const openFromBody = (path: string, id?: string) => {
    setAnchor(id === undefined ? undefined : { path, id })
    desk.open(path)
  }
  // 閉じたタブ（ファイルが消えて外れたタブも）の位置と、送る前に閉じたタブの送る先は忘れる
  createEffect(desk.tabs, (paths) => {
    for (const path of scrolls.keys()) if (!paths.includes(path)) scrolls.delete(path)
    const pending = untrack(anchor)
    if (pending && !paths.includes(pending.path)) setAnchor(undefined)
  })

  createEffect(desk.selected, (path) => {
    // 見出しへ送る途中のタブは、送る所（FilePreview）が位置を決める
    if (path === null || untrack(anchor)?.path === path) return
    // 中身が替わって縮んだ所で覚えた位置が上書きされる前に、戻す先を決めておく
    const top = scrolls.get(path) ?? 0
    const frame = requestAnimationFrame(() => island?.scrollTo?.({ top }))
    return () => cancelAnimationFrame(frame)
  })

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
              ref={(element) => {
                island = element
                element.addEventListener("scroll", rememberScroll, { passive: true })
              }}
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
                    {/* 右クリックのメニューは順 5 の続き（5d） */}
                    <For each={tabs()} keyed={(tab) => tab.path}>
                      {(tab) => (
                        // ほかのタブも隠して残す。移るたびに本文を読み直して上から描き直すと、戻した位置や送った先の前に、上の絵が出る
                        <TabsPanel
                          value={tab().path}
                          keepMounted
                          class={css({ outline: "none" })}
                          style={{ visibility: anchor()?.path === tab().path ? "hidden" : undefined }}
                        >
                          <Show when={links().byPath.get(tab().path)}>
                            {(doc) => (
                              <FilePreview
                                doc={doc()}
                                title={tab().title}
                                byPath={links().byPath}
                                onOpen={openFromBody}
                                onOpenUrl={openExternal}
                                anchor={
                                  anchor()?.path === tab().path && desk.selected() === tab().path ? anchor() : undefined
                                }
                                onAnchorDone={() => setAnchor(undefined)}
                              />
                            )}
                          </Show>
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
          class={cx(passThrough, css({ display: "flex", flexDirection: "column", paddingTop: "12" }))}
        >
          <Show when={inspected()}>
            {(content) => (
              // 行の地は、窓の側の縁から 8px。島の側は列の境目（8px）がその間になる（左の列と同じ）
              <aside
                aria-label="今のファイル"
                class={cx(
                  pressable,
                  css({ flex: "1", minHeight: "0", overflow: "auto", paddingInlineEnd: "2", paddingBottom: "2" }),
                )}
              >
                <DeskInspector
                  content={content()}
                  sections={sections}
                  onOpen={(path) => desk.open(path)}
                  onOpenUrl={openExternal}
                />
              </aside>
            )}
          </Show>
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
