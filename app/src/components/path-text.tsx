import { cx } from "../../styled-system/css"
import { pathText } from "../../styled-system/recipes"

// 折り返すときは / の後ろで折る。ハイフンの後ろでは折らない（行の終わりのハイフンが名前の一部か折り返しか
// 見分けにくい）。そのため、ハイフンとその次の 1 字を折れない塊にする。/ の後ろで折れない長い名前はどこでも折る（レシピ）。
// コピーした文字には何も足さない
export function PathText(props: { path: string; class?: string }) {
  return (
    <span class={cx(pathText(), props.class)}>
      {props.path.split("/").map((segment, i) => (i === 0 ? <Segment text={segment} /> : ["/", <wbr />, <Segment text={segment} />]))}
    </span>
  )
}

// 名前の終わりの拡張子は、前の語（8 字まで。その前のハイフンも）と折れない塊にする。行の終わりに「md」だけが残らないように
const TAIL = /-?[^-]{0,8}\.[^.-]+$/

function Segment(props: { text: string }) {
  const tail = props.text.match(TAIL)?.[0] ?? ""
  const head = props.text.slice(0, props.text.length - tail.length)
  return [
    ...head.split(/(-+.?)/).map((part, i) => (i % 2 === 1 ? <span data-keep="">{part}</span> : part)),
    tail && <span data-keep="">{tail}</span>,
  ]
}
