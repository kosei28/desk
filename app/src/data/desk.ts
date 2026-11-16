import { createMemo, createSignal, onCleanup } from "solid-js"
import { chooseRoot, getIndex, onTreeChanged } from "./commands"
import type { Doc, Index } from "./types"

export type Desk = ReturnType<typeof createDesk>

type Tabs = {
  /** 開いているファイルのルートからのパス。タブの並びの順 */
  paths: string[]
  /** 今のタブのパス。タブが無ければ null */
  active: string | null
}

const noTabs: Tabs = { paths: [], active: null }

/**
 * 開いているツリーの索引と、開いているファイルのタブ（アプリ全体で 1 組）。今のタブのファイルが、画面の選択。
 * owner（コンポーネントか createRoot）の中で作る。作るとすぐ索引をもらい、ツリーが変わるたびにもらい直す。
 * owner が消えると、変わった知らせを聞くのをやめる。
 */
export function createDesk() {
  const [index, setIndex] = createSignal<Index | null>(null)
  const [loaded, setLoaded] = createSignal(false)
  const [error, setError] = createSignal<string | null>(null)
  const [tabs, setTabs] = createSignal<Tabs>(noTabs)
  const byPath = createMemo(() => new Map<string, Doc>(index()?.docs.map((doc) => [doc.path, doc])))
  const selected = () => tabs().active
  const selectedDoc = createMemo(() => {
    const path = selected()
    return path === null ? undefined : byPath().get(path)
  })

  /** タブを外す。今のタブを外したら、右隣（無ければ左隣）を今のタブにする */
  function without(current: Tabs, gone: (path: string) => boolean): Tabs {
    const paths = current.paths.filter((path) => !gone(path))
    if (current.active === null || !gone(current.active)) return { paths, active: current.active }
    const at = current.paths.indexOf(current.active)
    const right = current.paths.slice(at + 1).find((path) => !gone(path))
    const left = current.paths.slice(0, at).reverse().find((path) => !gone(path))
    return { paths, active: right ?? left ?? null }
  }

  // 読み込みが重なったら、最後に頼んだものの結果だけを使う
  let latest = 0
  let root: string | undefined
  async function load() {
    const request = ++latest
    try {
      const next = await getIndex()
      if (request !== latest) return
      // タブはルートからのパスなので、別のツリーに移ったら全部閉じる。消えたファイルのタブも閉じる
      if (next?.root !== root) setTabs(noTabs)
      else if (next) {
        const present = new Set(next.docs.map((doc) => doc.path))
        setTabs((current) => without(current, (path) => !present.has(path)))
      }
      root = next?.root
      setIndex(next)
      setError(null)
    } catch (e) {
      if (request !== latest) return
      setError(String(e))
    } finally {
      if (request === latest) setLoaded(true)
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
    /** 最初の索引の読み込みが終わったか（もらえても、失敗しても true） */
    loaded,
    /** 最後に索引をもらうかツリーを開くのに失敗したときの説明。索引をもらえたら null に戻る */
    error,
    /** 開いているファイルのパス。タブの並びの順 */
    tabs: () => tabs().paths,
    /** 今のタブのファイルのルートからのパス。タブが無ければ null */
    selected,
    /** 今のタブのファイル。タブが無いか、そのパスがもうツリーに無ければ undefined */
    selectedDoc,
    /** ファイルを新しいタブで開き、今のタブの右に置いて切り替える。もう開いているなら、そのタブに切り替える */
    open(path: string) {
      setTabs((current) => {
        if (current.paths.includes(path)) return { paths: current.paths, active: path }
        const at = current.active === null ? current.paths.length : current.paths.indexOf(current.active) + 1
        return { paths: [...current.paths.slice(0, at), path, ...current.paths.slice(at)], active: path }
      })
    },
    /** タブを閉じる。最後のタブを閉じると、今のタブは無くなる */
    close(path: string) {
      setTabs((current) => without(current, (p) => p === path))
    },
    /** 今のタブを step だけ隣に移す。端からは反対の端に回る */
    cycle(step: 1 | -1) {
      setTabs((current) => {
        if (current.active === null || current.paths.length < 2) return current
        const n = current.paths.length
        const at = current.paths.indexOf(current.active)
        return { paths: current.paths, active: current.paths[(at + step + n) % n] }
      })
    },
    /** フォルダーを選んでツリーを開く */
    choose,
  }
}
