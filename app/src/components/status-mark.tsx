import { Match, Switch } from "solid-js"
import { statusMark } from "../../styled-system/recipes"

export type Status = "backlog" | "todo" | "inprogress" | "done" | "dropped"

// 12px の円 1 種類。輪から塗りへ進み、灰を共有する backlog は点線、dropped は斜線で分ける
export function StatusMark(props: { status: Status }) {
  return (
    <svg
      class={statusMark({ status: props.status })}
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      role="img"
      aria-label={props.status}
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

export function HumanMark() {
  return (
    <svg viewBox="0 0 12 12" fill="currentColor" role="img" aria-label="人が要る">
      <circle cx="6" cy="3.5" r="2.5" />
      <path d="M1.5 11.25a4.5 4.5 0 0 1 9 0z" />
    </svg>
  )
}
