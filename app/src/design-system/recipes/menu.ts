import { defineSlotRecipe } from "@pandacss/dev"
import { floatStyles } from "./styles"

export const menuRecipe = defineSlotRecipe({
  className: "menu",
  description: "UI SDK の Menu。右クリックで開く。浮かぶ面に行を並べ、まとまりの間は線で区切る",
  slots: ["content", "item", "separator", "groupLabel"],
  base: {
    content: {
      ...floatStyles,
      display: "flex",
      flexDirection: "column",
      minWidth: "menu",
      padding: "1",
      borderRadius: "float",
    },
    item: {
      display: "flex",
      alignItems: "center",
      gap: "2",
      height: "row",
      paddingInline: "3",
      borderRadius: "item",
      textStyle: "ui",
      color: "fg",
      cursor: "default",
      userSelect: "none",
      outline: "none",
      whiteSpace: "nowrap",
      _highlighted: { bg: "fill" },
      _disabled: { color: "fg.muted" },
    },
    separator: {
      height: "token(borderWidths.hairline)",
      marginBlock: "1",
      marginInline: "3",
      bg: "separator",
    },
    groupLabel: {
      display: "flex",
      alignItems: "end",
      height: "row",
      paddingInline: "3",
      paddingBlockEnd: "1",
      textStyle: "group",
      color: "fg.muted",
    },
  },
})
