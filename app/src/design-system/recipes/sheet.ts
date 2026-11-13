import { defineRecipe } from "@pandacss/dev"

export const sheetRecipe = defineRecipe({
  className: "sheet",
  description: "紙。いま読んでいるファイルを載せる、机より透けの少ない面",
  base: {
    bg: "paper",
    color: "fg",
    borderRadius: "sheet",
    boxShadow: "paper",
    overflow: "auto",
    paddingBlock: "10",
    paddingInline: "12",
  },
})
