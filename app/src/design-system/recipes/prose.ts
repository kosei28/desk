import { defineRecipe } from "@pandacss/dev"
import { inlineLinkStyles } from "./link"

// チェックボックスの印。地はアクセントで、明るい地のライトは白、明るい紫のダークは濃い墨の線を引く
const check = (stroke: string) =>
  `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16' fill='none' stroke='${stroke}' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m4.5 8.25 2.25 2.25 4.75-4.75'/%3E%3C/svg%3E")`

export const proseRecipe = defineRecipe({
  className: "prose",
  description:
    "Markdown の本文。読む面に置く。見出しは丸い書体で、h1 28・h2 20・h3 16、h4 から下は本文の大きさの太字。行は 40em まで。区切りの線は表の行・区切り・引用だけに引く",
  base: {
    maxWidth: "measure",
    textStyle: "reading",
    color: "fg",
    // 間は前の要素との間だけで取り、見出しの後ろは詰める
    "& > * + *": { marginBlockStart: "4" },
    "& > * + h2": { marginBlockStart: "10" },
    "& > * + h3": { marginBlockStart: "8" },
    "& > * + :is(h4, h5, h6)": { marginBlockStart: "6" },
    "& > :is(h1, h2, h3, h4, h5, h6) + *": { marginBlockStart: "2" },
    "& h1": { textStyle: "title" },
    "& h2": { textStyle: "heading" },
    "& h3": { textStyle: "subheading" },
    "& :is(h4, h5, h6)": { fontWeight: "bold" },
    "& strong": { fontWeight: "bold" },
    "& a": inlineLinkStyles,
    // 本文の中のコード。文字の大きさは本文に対する比で決め、行の高さを崩さない
    "& :not(pre) > code": {
      paddingInline: "0.3em",
      paddingBlock: "0.1em",
      borderRadius: "inline",
      bg: "fill",
      fontFamily: "mono",
      fontSize: "0.875em",
      boxDecorationBreak: "clone",
    },
    "& pre": {
      overflowX: "auto",
      paddingBlock: "3",
      paddingInline: "4",
      borderRadius: "surface",
      bg: "fill",
      fontFamily: "mono",
      fontSize: "0.875em",
      lineHeight: "ui",
    },
    "& :is(ul, ol)": { paddingInlineStart: "6" },
    "& ul": { listStyleType: "disc" },
    "& ol": { listStyleType: "decimal" },
    "& li::marker": { color: "fg.muted" },
    "& li + li, & li > :is(ul, ol)": { marginBlockStart: "1" },
    // 手順のチェックボックスは表示だけで、押せない。角の丸い四角にして、status の印（円）と分ける
    "& .task-list-item": { listStyleType: "none" },
    "& .task-list-item > input[type=checkbox]": {
      appearance: "none",
      display: "inline-block",
      marginInlineStart: "-6",
      marginInlineEnd: "2",
      width: "icon",
      height: "icon",
      verticalAlign: "-0.15em",
      borderRadius: "inline",
      borderWidth: "mark",
      borderStyle: "solid",
      borderColor: "fg.muted",
      pointerEvents: "none",
      "&:checked": {
        borderColor: "accent",
        bg: "accent",
        backgroundImage: check("white"),
        backgroundSize: "full",
        _dark: { backgroundImage: check("%23161325") },
      },
    },
    "& .task-list-item:has(> input:checked)": { color: "fg.muted" },
    "& blockquote": {
      paddingInlineStart: "4",
      borderInlineStartWidth: "quote",
      borderInlineStartStyle: "solid",
      borderInlineStartColor: "separator",
      color: "fg.muted",
    },
    "& hr": {
      height: "token(borderWidths.hairline)",
      marginBlock: "8",
      border: "none",
      bg: "separator",
    },
    "& table": { width: "full", borderCollapse: "collapse", lineHeight: "ui" },
    "& :is(th, td)": {
      paddingBlock: "2",
      paddingInlineEnd: "4",
      textAlign: "start",
      verticalAlign: "top",
      borderBlockEndWidth: "hairline",
      borderBlockEndStyle: "solid",
      borderBlockEndColor: "separator",
    },
    "& th": { fontWeight: "bold" },
    "& img": { maxWidth: "full", height: "auto", borderRadius: "surface" },
  },
})
