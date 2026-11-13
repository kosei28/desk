import type { JSX } from "@solidjs/web"
import { cx } from "../../styled-system/css"
import { sheet } from "../../styled-system/recipes"

type SheetProps = {
  class?: string
  children?: JSX.Element
}

export function Sheet(props: SheetProps) {
  return <article class={cx(sheet(), props.class)}>{props.children}</article>
}
