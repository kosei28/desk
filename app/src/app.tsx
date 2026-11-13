import { createMemo, createSignal, Show } from "solid-js"
import { css } from "../styled-system/css"
import { Sheet } from "./components/sheet"
import { FileTree } from "./features/file-tree/file-tree"
import { sampleFiles } from "./sample-tree"

export function App() {
  const [selected, setSelected] = createSignal<string | null>(null)
  const selectedFile = createMemo(() => sampleFiles.find((file) => file.path === selected()))

  return (
    <div
      class={css({
        display: "grid",
        gridTemplateColumns: "token(sizes.sidebar) minmax(0, 1fr)",
        height: "full",
      })}
    >
      <nav
        aria-label="ツリー"
        class={css({ display: "flex", flexDirection: "column", minHeight: "0" })}
      >
        <div class={css({ flexShrink: "0", height: "titlebar" })} />
        <div class={css({ flex: "1", minHeight: "0", overflow: "auto", padding: "2", paddingTop: "0" })}>
          <FileTree files={sampleFiles} selected={selected()} onSelect={setSelected} />
        </div>
      </nav>
      <div class={css({ display: "grid", minHeight: "0", padding: "2", paddingInlineStart: "0" })}>
        <Sheet>
          <Show
            when={selectedFile()}
            fallback={
              <p
                class={css({
                  display: "grid",
                  placeItems: "center",
                  height: "full",
                  color: "fg.muted",
                })}
              >
                ファイルを選ぶと、ここに開く
              </p>
            }
          >
            {(file) => (
              <header class={css({ display: "flex", flexDirection: "column", gap: "2", maxWidth: "measure" })}>
                <p class={css({ textStyle: "mono", color: "fg.muted" })}>{file().path}</p>
                <h1 class={css({ textStyle: "display" })}>{file().title}</h1>
              </header>
            )}
          </Show>
        </Sheet>
      </div>
    </div>
  )
}
