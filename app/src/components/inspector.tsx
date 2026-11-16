import type { JSX } from "@solidjs/web"
import { Collapsible, CollapsiblePanel, CollapsibleTrigger } from "@ui-sdk/solid/collapsible"
import { For, Show } from "solid-js"
import { inspector, link } from "../../styled-system/recipes"
import { ChevronRight } from "./icons"
import { PathText } from "./path-text"
import { HumanMark, StatusMark, type Status } from "./status-mark"

const classes = inspector()

// 右の列。いちばん上（InspectorHead）の下に、見出し（InspectorSection）を並べる。中身の無い項目と見出しは、呼ぶ側が出さない
export function Inspector(props: { children: JSX.Element }) {
  return <div class={classes.root}>{props.children}</div>
}

// いちばん上。見出しが無く、畳まない。パス（名前を付けない）の下に、頭（InspectorTable）と札（InspectorPills）を置く
export function InspectorHead(props: { path: string; children?: JSX.Element }) {
  return (
    <div class={classes.head}>
      <div class={classes.path}>
        <PathText path={props.path} />
      </div>
      {props.children}
    </div>
  )
}

// 文字の値（頭とコード）の表。項目の名前と値の 2 列で、名前の列の幅はこの表にいま出ている名前のいちばん長いものに合わせる
export function InspectorTable(props: { children: JSX.Element }) {
  return <dl class={classes.table}>{props.children}</dl>
}

export function InspectorRow(props: { label: string; children: JSX.Element }) {
  return (
    <div class={classes.row}>
      <dt class={classes.name}>{props.label}</dt>
      <dd class={classes.value}>{props.children}</dd>
    </div>
  )
}

// URL の値。/ の後ろで折る（ハイフンでも折れる）。指したときだけ下線を引く。
// 押すと href へ移らずに onOpen を呼ぶ（webview の中で外のページへ移らないように。開くのは呼ぶ側）
export function InspectorUrl(props: { href: string; onOpen: () => void }) {
  return (
    <a
      href={props.href}
      class={link({ tone: "quiet" })}
      onClick={(event) => {
        event.preventDefault()
        props.onOpen()
      }}
    >
      {props.href.split("/").map((part, i) => (i === 0 ? part : ["/", <wbr />, part]))}
    </a>
  )
}

// 札の並び。ステータス → 要対応 → タグの順に置く
export function InspectorPills(props: { children: JSX.Element }) {
  return <div class={classes.pills}>{props.children}</div>
}

// ステータスの札。白の印と値の文字を、status の色相の色で透かした地に入れる
export function InspectorStatus(props: { status: Status }) {
  return (
    <span class={classes.pill} data-status={props.status}>
      <StatusMark status={props.status} decorative plain />
      {props.status}
    </span>
  )
}

// needs_human が true のときの札。人の印と「要対応」
export function InspectorHuman() {
  return (
    <span class={classes.pill}>
      <HumanMark decorative />
      要対応
    </span>
  )
}

// タグの札。1 つの値に 1 つ、値だけ
export function InspectorTag(props: { children: JSX.Element }) {
  return <span class={classes.pill}>{props.children}</span>
}

// 見出しと、その中身。どの見出しも同じ形で、行の幅いっぱいを押すと畳む。矢印は右端にいつも出し、件数は出さない。
// 開け閉めは呼ぶ側が持つ（見出しの言葉ごとに覚え、ほかのファイルに移っても保つため）
export function InspectorSection(props: {
  title: string
  open: boolean
  onOpenChange: (open: boolean) => void
  children: JSX.Element
}) {
  return (
    <section class={classes.section} aria-label={props.title}>
      <Collapsible open={props.open} onOpenChange={props.onOpenChange}>
        <h2>
          <CollapsibleTrigger class={classes.trigger}>
            {props.title}
            <span class={classes.indicator}>
              <ChevronRight />
            </span>
          </CollapsibleTrigger>
        </h2>
        <CollapsiblePanel class={classes.panel}>{props.children}</CollapsiblePanel>
      </Collapsible>
    </section>
  )
}

// 見出しの中の、名前の付いた一覧（InspectorList）の並び
export function InspectorGroups(props: { children: JSX.Element }) {
  return <div class={classes.groups}>{props.children}</div>
}

// ファイルの行の一覧。label があれば名前を上に置き、空なら「[]」と出す。
// 行は key（パス）で見分け、一覧がもらい直されても同じ行は作り直さない（フォーカスを保つ）。children には行の値の accessor を渡す
export function InspectorList<T>(props: {
  label?: string
  each: readonly T[]
  key: (item: T) => string
  children: (item: () => T) => JSX.Element
}) {
  const items = () => (
    <div class={classes.items}>
      <For each={props.each} keyed={props.key}>
        {(item) => props.children(item)}
      </For>
    </div>
  )
  return (
    <Show when={props.label} fallback={items()}>
      {(label) => (
        <div class={classes.list}>
          <span class={classes.listName}>{label()}</span>
          <Show when={props.each.length > 0} fallback={<span class={classes.empty}>[]</span>}>
            {items()}
          </Show>
        </div>
      )}
    </Show>
  )
}

// ファイルの 1 行。左の列の行と同じで、タスクなら頭に status の印、人が要るなら右端に人の印。見出しは省かずに折り返す。
// place は、場所が行から分からないときに添える場所（プロジェクトの名前・archive）。
// href はいつも要る。href の無いリンクはキーボードで届かず、指したときの地も付かないため。
// 押すと href へ移らずに onOpen を呼ぶ。開き方は ⌘ の有る無しで変えない（いつも新しいタブ。開いているならそのタブ）。
// キーでは、左の列の行と同じく Enter と Space で開く（Space で列を送らない。Enter は keydown で取り、リンクの既定の click を起こさない）
export function InspectorItem(props: {
  title: string
  status?: Status
  needsHuman?: boolean
  place?: string
  href: string
  path?: string
  onOpen: () => void
}) {
  return (
    <a
      class={classes.item}
      href={props.href}
      data-path={props.path}
      onClick={(event) => {
        event.preventDefault()
        props.onOpen()
      }}
      onKeyDown={(event) => {
        if (event.isComposing || (event.key !== "Enter" && event.key !== " ")) return
        event.preventDefault()
        props.onOpen()
      }}
    >
      <Show when={props.status}>
        {(status) => (
          <span class={classes.lead}>
            <StatusMark status={status()} />
          </span>
        )}
      </Show>
      <span class={classes.text}>
        {props.title}
        <Show when={props.place}>{(place) => <span class={classes.place}>{place()}</span>}</Show>
      </span>
      <Show when={props.needsHuman}>
        <span class={classes.trail}>
          <HumanMark />
        </span>
      </Show>
    </a>
  )
}
