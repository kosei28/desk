import { createMemo, createSignal, onCleanup } from "solid-js"
import { chooseRoot, getIndex, onTreeChanged } from "./commands"
import type { Doc, Index } from "./types"

export type Desk = ReturnType<typeof createDesk>

/**
 * 開いているツリーの索引と、選んでいるファイル（アプリ全体で 1 つ）。
 * owner（コンポーネントか createRoot）の中で作る。作るとすぐ索引をもらい、ツリーが変わるたびにもらい直す。
 * owner が消えると、変わった知らせを聞くのをやめる。
 */
export function createDesk() {
  const [index, setIndex] = createSignal<Index | null>(null)
  const [error, setError] = createSignal<string | null>(null)
  const [selected, setSelected] = createSignal<string | null>(null)
  const byPath = createMemo(() => new Map<string, Doc>(index()?.docs.map((doc) => [doc.path, doc])))
  const selectedDoc = createMemo(() => {
    const path = selected()
    return path === null ? undefined : byPath().get(path)
  })

  // 読み込みが重なったら、最後に頼んだものの結果だけを使う
  let latest = 0
  let root: string | undefined
  async function load() {
    const request = ++latest
    try {
      const next = await getIndex()
      if (request !== latest) return
      // 選んでいるのはルートからのパスなので、別のツリーに移ったら選び直す
      if (next?.root !== root) setSelected(null)
      root = next?.root
      setIndex(next)
      setError(null)
    } catch (e) {
      if (request !== latest) return
      setError(String(e))
    }
  }

  async function choose() {
    try {
      await chooseRoot()
    } catch (e) {
      setError(String(e))
      return
    }
    await load()
  }

  const unlisten = onTreeChanged(() => void load())
  onCleanup(() => void unlisten.then((stop) => stop()))
  void load()

  return {
    /** 索引。もらう前と、ツリーを開いていないときは null */
    index,
    /** 最後に索引をもらうかツリーを開くのに失敗したときの説明。索引をもらえたら null に戻る */
    error,
    /** 選んでいるファイルのルートからのパス */
    selected,
    /** 選んでいるファイル。選んでいないか、そのパスがもうツリーに無ければ undefined */
    selectedDoc,
    select: (path: string | null) => setSelected(path),
    /** フォルダーを選んでツリーを開く */
    choose,
  }
}
