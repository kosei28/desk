import { defineSlotRecipe } from "@pandacss/dev"

export const treeRecipe = defineSlotRecipe({
  className: "tree",
  description: "UI SDK の Tree。lead は行の頭の 16px の枠で、フォルダーは矢印、タスクは status の印を置く。印の無いファイルは枠を持たず、文字を兄弟の矢印の線にそろえる",
  slots: ["root", "item", "label", "lead", "indicator", "text", "trail", "group"],
  base: {
    root: {
      display: "flex",
      flexDirection: "column",
      textStyle: "ui",
      color: "fg",
    },
    item: {
      outline: "none",
    },
    label: {
      display: "flex",
      alignItems: "center",
      gap: "1.5",
      height: "row",
      paddingInline: "2",
      borderRadius: "full",
      cursor: "default",
      userSelect: "none",
      transitionProperty: "background-color, color",
      transitionDuration: "fast",
      transitionTimingFunction: "standard",
      _hover: { bg: "paper.hover" },
      _selected: {
        bg: "paper",
        color: "accent",
        _hover: { bg: "paper" },
      },
      // 輪は行の内側に描く。外に出すと、下の行の地（選択の紙）に下の線が隠れ、スクロールの入れ物に一番上の行の上の線が切られる
      "[data-slot=tree-item]:focus-visible > &": {
        outlineWidth: "focus",
        outlineStyle: "solid",
        outlineColor: "accent",
        outlineOffset: "calc(-1 * token(borderWidths.focus))",
      },
      "@media (prefers-reduced-motion: reduce)": { transitionDuration: "0s" },
    },
    lead: {
      display: "inline-flex",
      alignItems: "center",
      justifyContent: "center",
      flexShrink: "0",
      width: "icon",
      height: "icon",
    },
    indicator: {
      display: "inline-flex",
      width: "icon",
      height: "icon",
      color: "fg.muted",
      transitionProperty: "transform",
      transitionDuration: "base",
      transitionTimingFunction: "standard",
      _expanded: { transform: "rotate(90deg)" },
      "@media (prefers-reduced-motion: reduce)": { transitionDuration: "0s" },
      "& svg": { width: "full", height: "full" },
    },
    text: {
      flex: "1",
      minWidth: "0",
      overflow: "hidden",
      textOverflow: "ellipsis",
      whiteSpace: "nowrap",
      // 印の無いファイルは枠を持たず、文字が行の頭に来る。兄弟のフォルダーの閉じた矢印の線（枠の左から 5.6px）にそろえて 6px 入れる
      "&:first-child": { marginInlineStart: "1.5" },
    },
    trail: {
      display: "inline-flex",
      flexShrink: "0",
      color: "fg.muted",
      "& svg": { width: "mark", height: "mark" },
    },
    group: {
      display: "flex",
      flexDirection: "column",
      paddingInlineStart: "treeIndent",
    },
  },
})
