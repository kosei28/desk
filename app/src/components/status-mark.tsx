import { Match, Switch } from "solid-js"
import { statusMark } from "../../styled-system/recipes"

export type Status = "backlog" | "todo" | "inprogress" | "done" | "dropped"

// 12px の円 1 種類。色で見分け、円の塗り方で進み具合を補う。backlog は点線の輪、todo は輪、inprogress は輪と右半分の塗り、
// done は塗りの円、dropped は輪と斜線
// decorative は、隣に status の名前の文字があるとき（札）に付け、名前を 2 度読ませない。
// plain は status の色を付けず、置いた所の文字の色を継ぐ（ステータスの札の白の印）
export function StatusMark(props: { status: Status; decorative?: boolean; plain?: boolean }) {
  return (
    <svg
      class={statusMark({ status: props.plain ? undefined : props.status })}
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      role={props.decorative ? undefined : "img"}
      aria-label={props.decorative ? undefined : props.status}
      aria-hidden={props.decorative ? "true" : undefined}
    >
      <Switch>
        <Match when={props.status === "backlog"}>
          <circle cx="6" cy="6" r="5.25" stroke-dasharray="2.06 2.06" />
        </Match>
        <Match when={props.status === "todo"}>
          <circle cx="6" cy="6" r="5.25" />
        </Match>
        <Match when={props.status === "inprogress"}>
          <circle cx="6" cy="6" r="5.25" />
          <path d="M6 2.5a3.5 3.5 0 0 1 0 7z" fill="currentColor" stroke="none" />
        </Match>
        <Match when={props.status === "done"}>
          <circle cx="6" cy="6" r="6" fill="currentColor" stroke="none" />
        </Match>
        <Match when={props.status === "dropped"}>
          <circle cx="6" cy="6" r="5.25" />
          <path d="m2.5 9.5 7-7" stroke-linecap="round" />
        </Match>
      </Switch>
    </svg>
  )
}

// decorative は、隣に「要対応」の文字があるとき（札）に付ける
export function HumanMark(props: { decorative?: boolean }) {
  return (
    <svg
      viewBox="0 0 12 12"
      fill="currentColor"
      role={props.decorative ? undefined : "img"}
      aria-label={props.decorative ? undefined : "人が要る"}
      aria-hidden={props.decorative ? "true" : undefined}
    >
      <circle cx="6" cy="3.5" r="2.5" />
      <path d="M1.5 11.25a4.5 4.5 0 0 1 9 0z" />
    </svg>
  )
}
