import type { JSX } from "@solidjs/web"
import { cx } from "../../styled-system/css"
import { surface } from "../../styled-system/recipes"

type SurfaceProps = {
  class?: string
  children?: JSX.Element
}

export function Surface(props: SurfaceProps) {
  return <article class={cx(surface(), props.class)}>{props.children}</article>
}
