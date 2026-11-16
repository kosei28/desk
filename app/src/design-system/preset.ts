import { definePreset } from "@pandacss/dev"
import { statusMarkRecipe } from "./recipes/status-mark"
import { surfaceRecipe } from "./recipes/surface"
import { treeRecipe } from "./recipes/tree"
import { semanticTokens } from "./semantic-tokens"
import { textStyles } from "./text-styles"
import { tokens } from "./tokens"

export const deskPreset = definePreset({
  name: "desk",
  globalCss: {
    body: {
      bg: "background",
      color: "fg",
      textStyle: "ui",
      WebkitFontSmoothing: "antialiased",
    },
  },
  theme: {
    tokens,
    semanticTokens,
    textStyles,
    recipes: {
      surface: surfaceRecipe,
      statusMark: statusMarkRecipe,
    },
    slotRecipes: { tree: treeRecipe },
  },
})
