import { definePreset } from "@pandacss/dev"
import { sheetRecipe } from "./recipes/sheet"
import { statusMarkRecipe } from "./recipes/status-mark"
import { treeRecipe } from "./recipes/tree"
import { semanticTokens } from "./semantic-tokens"
import { textStyles } from "./text-styles"
import { tokens } from "./tokens"

export const deskPreset = definePreset({
  name: "desk",
  globalCss: {
    body: {
      bg: "ground",
      color: "fg",
      textStyle: "ui",
      WebkitFontSmoothing: "antialiased",
    },
  },
  theme: {
    tokens,
    semanticTokens,
    textStyles,
    recipes: { sheet: sheetRecipe, statusMark: statusMarkRecipe },
    slotRecipes: { tree: treeRecipe },
  },
})
