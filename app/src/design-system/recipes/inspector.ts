import { defineSlotRecipe } from "@pandacss/dev"
import { focusRingStyles, rowTransitionStyles } from "./styles"

const STATUSES = ["backlog", "todo", "inprogress", "done", "dropped"] as const

export const inspectorRecipe = defineSlotRecipe({
  className: "inspector",
  description:
    "右の列。いちばん上（見出しなし・畳まない）にパス・頭（このファイルの文字の値）・札を置き、その下に意味で分けた見出し（コード・親子と依存・関連ファイル・リンク・他からのリンク）を同じ形で並べる。見出しはどれも畳める。区切りの線は引かず、余白で分ける。どの行も、最初のものを列の縁から 16px の行の頭にそろえる",
  slots: [
    "root",
    "head",
    "path",
    "table",
    "row",
    "name",
    "value",
    "pills",
    "pill",
    "section",
    "trigger",
    "indicator",
    "panel",
    "groups",
    "list",
    "listName",
    "empty",
    "items",
    "item",
    "lead",
    "text",
    "place",
    "trail",
  ],
  base: {
    // 列の縁から 8px の所に行の地を置き、行の地の中 8px に文字を置く。左の列と同じ組み立て。いちばん上と見出しの間は 12px
    root: {
      display: "flex",
      flexDirection: "column",
      gap: "3",
      paddingInline: "2",
      color: "fg",
      textStyle: "ui",
    },
    // いちばん上。パス → 頭 → 札で、間は 8px
    head: {
      display: "flex",
      flexDirection: "column",
      gap: "2",
    },
    path: {
      paddingInline: "2",
    },
    // 文字の値（頭とコード）。項目の名前と値の 2 列。名前の列の幅は、この表にいま出ている名前のいちばん長いものに合わせる
    table: {
      display: "grid",
      gridTemplateColumns: "max-content minmax(0, 1fr)",
      columnGap: "2",
      rowGap: "1.5",
      alignItems: "baseline",
      paddingInline: "2",
    },
    // 名前と値の組。表の列にそのまま並べる
    row: {
      display: "contents",
    },
    // 項目の名前（頭・コードの名前と、見出しの中の名前）。背景の上の fg.label より 1 段薄い
    name: {
      textStyle: "label",
      color: "fg.name",
    },
    value: {
      minWidth: "0",
      overflowWrap: "anywhere",
    },
    // 札（ステータス・人が要る・タグ）。名前を付けない丸い形
    pills: {
      display: "flex",
      flexWrap: "wrap",
      gap: "1.5",
      paddingInline: "2",
    },
    // 札の地は読む面と同じ地で、縁の線も影も付けない（選んでいる印と役を分ける）。ステータスの札は、地を status の色相の色で
    // 透かして塗り、印と文字を白にする
    pill: {
      display: "inline-flex",
      alignItems: "center",
      gap: "1",
      height: "pill",
      // 情報設計のスクショの札（縁の線の幅 1px を残して透明にしていた）と同じ大きさにする
      paddingInline: "[9px]",
      borderRadius: "full",
      bg: "pill",
      textStyle: "label",
      color: "fg",
      whiteSpace: "nowrap",
      "& svg": { width: "mark", height: "mark", flexShrink: "0" },
      "&[data-status]": { color: "statusPill.fg" },
      ...Object.fromEntries(STATUSES.map((s) => [`&[data-status=${s}]`, { bg: `statusPill.${s}` }])),
    },
    section: {
      display: "flex",
      flexDirection: "column",
    },
    // 見出し。行の幅いっぱいが押せる所で、押すと畳む。矢印は右端にいつも出す。件数は出さない
    trigger: {
      ...rowTransitionStyles,
      display: "flex",
      alignItems: "center",
      gap: "1.5",
      width: "full",
      height: "row",
      paddingInline: "2",
      borderRadius: "full",
      textStyle: "group",
      color: "fg",
      textAlign: "start",
      cursor: "default",
      userSelect: "none",
      outline: "none",
      _hover: { bg: "fill" },
      _focusVisible: focusRingStyles,
    },
    indicator: {
      display: "inline-flex",
      flexShrink: "0",
      width: "icon",
      height: "icon",
      marginInlineStart: "auto",
      color: "fg.label",
      transitionProperty: "transform",
      transitionDuration: "base",
      transitionTimingFunction: "standard",
      "[aria-expanded=true] > &": { transform: "rotate(90deg)" },
      "@media (prefers-reduced-motion: reduce)": { transitionDuration: "0s" },
      "& svg": { width: "full", height: "full" },
    },
    panel: {
      paddingBlockStart: "1",
    },
    // 見出しの中の、名前の付いた一覧の並び（親子と依存・関連ファイル）
    groups: {
      display: "flex",
      flexDirection: "column",
      gap: "2",
    },
    // 名前の下にファイルの行。名前は押せない
    list: {
      display: "flex",
      flexDirection: "column",
    },
    listName: {
      paddingInline: "2",
      textStyle: "label",
      color: "fg.name",
    },
    // 空のリスト。「[]」と出す
    empty: {
      paddingInline: "2",
      paddingBlockStart: "[2px]",
    },
    items: {
      display: "flex",
      flexDirection: "column",
    },
    // ファイルの 1 行。左の列の行と同じ地（指すと fill）。見出しは省かずに折り返すので、高さは 28px から伸びる。
    // 角は 1 行の高さの半分にし、折り返した行の地が縦長の楕円にならず、頭と右端の印が地の中に収まるようにする
    item: {
      ...rowTransitionStyles,
      display: "flex",
      alignItems: "start",
      gap: "1.5",
      minHeight: "row",
      paddingBlock: "1",
      paddingInline: "2",
      borderRadius: "row",
      color: "fg",
      textDecoration: "none",
      cursor: "default",
      userSelect: "none",
      outline: "none",
      _hover: { bg: "fill" },
      "&[data-menu-open]": { bg: "fill" },
      _focusVisible: focusRingStyles,
    },
    // status の印。1 行目の文字の縦の中央に置く。印の無い行は枠を持たず、文字を行の頭から始める
    lead: {
      display: "inline-flex",
      alignItems: "center",
      flexShrink: "0",
      height: "[1lh]",
    },
    text: {
      flex: "1",
      minWidth: "0",
      overflowWrap: "anywhere",
    },
    // 場所が行から分からないときの添え書き（プロジェクトの名前・archive）
    place: {
      marginInlineStart: "1.5",
      color: "fg.label",
    },
    trail: {
      display: "inline-flex",
      alignItems: "center",
      flexShrink: "0",
      height: "[1lh]",
      color: "fg.label",
      "& svg": { width: "mark", height: "mark" },
    },
  },
})
