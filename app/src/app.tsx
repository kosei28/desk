import { createMemo, createSignal, Show } from "solid-js"
import { css, cx } from "../styled-system/css"
import { Surface } from "./components/surface"
import { FileTree } from "./features/file-tree/file-tree"
import { dragRegion } from "./features/window/drag-region"
import { sampleFiles } from "./sample-tree"

export function App() {
  const [selected, setSelected] = createSignal<string | null>(null)
  const selectedFile = createMemo(() => sampleFiles.find((file) => file.path === selected()))

  return (
    <div
      class={css({
        display: "grid",
        gridTemplateColumns: "token(sizes.sidebar) minmax(0, 1fr)",
        gridTemplateRows: "token(sizes.titlebar) minmax(0, 1fr)",
        height: "full",
      })}
    >
      {/* 帯はウィンドウの幅いっぱいの層。読む面はその上に載り、読む面に覆われていない所（ツリーの上、読む面の上と右の余白）で窓を動かす */}
      <div ref={dragRegion} class={css({ gridColumn: "1 / -1", gridRow: "1", zIndex: "titlebar" })} />
      <nav
        aria-label="ツリー"
        class={css({ gridColumn: "1", gridRow: "2", minHeight: "0", overflow: "auto", padding: "2", paddingTop: "0" })}
      >
        <FileTree files={sampleFiles} selected={selected()} onSelect={setSelected} />
      </nav>
      <div
        class={css({
          display: "grid",
          gridColumn: "2",
          gridRow: "1 / -1",
          minHeight: "0",
          padding: "2",
          paddingInlineStart: "0",
        })}
      >
        <Surface class={cx(css({ zIndex: "surface" }), !selectedFile() && css({ alignContent: "center" }))}>
          <Show
            when={selectedFile()}
            fallback={
              <p class={css({ textAlign: "center", color: "fg.muted" })}>
                ファイルを選ぶと、ここに開く
              </p>
            }
          >
            {(file) => (
              <header>
                <h1 class={css({ textStyle: "title" })}>{file().title}</h1>
              </header>
            )}
          </Show>
        </Surface>
      </div>
    </div>
  )
}
