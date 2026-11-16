import { defineSlotRecipe } from "@pandacss/dev"
import { focusRingStyles, rowTransitionStyles } from "./styles"

export const treeRecipe = defineSlotRecipe({
  className: "tree",
  description:
    "UI SDK の Tree。左の列に、ディレクトリの構造そのままに置く。行の頭は 16px の枠（lead）1 つで、フォルダーは開閉の矢印、タスクは status の印を置く。印の無いファイルは枠を持たず、文字を 6px 入れる。字下げは 1 段 22px（枠と、枠の後ろの間）で、子の行の枠が親の行の文字の頭に来る",
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
      ...rowTransitionStyles,
      display: "flex",
      alignItems: "center",
      gap: "1.5",
      height: "treeRow",
      paddingInline: "2",
      borderRadius: "full",
      cursor: "default",
      userSelect: "none",
      _hover: { bg: "fill" },
      // 選んでいる行は、選んでいるタブと同じ地にする。今のタブで開いているファイルがここ
      _selected: {
        bg: "selected",
        boxShadow: "raised",
        _hover: { bg: "selected" },
      },
      // 右クリックのメニューを開いている行。指したときと同じ地にして、どの行のメニューかを示す
      "&[data-menu-open]": { bg: "fill" },
      "[data-slot=tree-item]:is(:focus-visible, [data-focus-visible]) > &": focusRingStyles,
    },
    // 行の頭の枠。フォルダーなら開閉の矢印、タスクなら status の印（12px）を置く
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
      color: "fg.label",
      transitionProperty: "transform",
      transitionDuration: "base",
      transitionTimingFunction: "standard",
      _expanded: { transform: "rotate(90deg)" },
      "@media (prefers-reduced-motion: reduce)": { transitionDuration: "0s" },
      "& svg": { width: "full", height: "full" },
    },
    // 枠の無い行（印の無いファイル）は、文字を 6px 入れる
    text: {
      "&:first-child": { marginInlineStart: "1.5" },
      flex: "1",
      minWidth: "0",
      overflow: "hidden",
      textOverflow: "ellipsis",
      whiteSpace: "nowrap",
    },
    trail: {
      display: "inline-flex",
      flexShrink: "0",
      color: "fg.label",
      "& svg": { width: "mark", height: "mark" },
    },
    group: {
      display: "flex",
      flexDirection: "column",
      paddingInlineStart: "treeIndent",
    },
  },
})
