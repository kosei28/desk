import type { JSX } from "@solidjs/web"
import { cx } from "../../styled-system/css"
import { surface } from "../../styled-system/recipes"

type SurfaceProps = {
  class?: string
  /** 面の要素（スクロールする所）を受け取る */
  ref?: (element: HTMLElement) => void
  children?: JSX.Element
}

export function Surface(props: SurfaceProps) {
  return (
    <article ref={(element) => props.ref?.(element)} class={cx(surface(), props.class)}>
      {props.children}
    </article>
  )
}
