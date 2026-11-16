import { definePreset } from "@pandacss/dev"
import { iconButtonRecipe } from "./recipes/icon-button"
import { inspectorRecipe } from "./recipes/inspector"
import { linkRecipe } from "./recipes/link"
import { menuRecipe } from "./recipes/menu"
import { pathTextRecipe } from "./recipes/path-text"
import { proseRecipe } from "./recipes/prose"
import { resizeHandleRecipe } from "./recipes/resize-handle"
import { statusMarkRecipe } from "./recipes/status-mark"
import { surfaceRecipe } from "./recipes/surface"
import { tabsRecipe } from "./recipes/tabs"
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
      link: linkRecipe,
      pathText: pathTextRecipe,
      iconButton: iconButtonRecipe,
      resizeHandle: resizeHandleRecipe,
      prose: proseRecipe,
    },
    slotRecipes: { tree: treeRecipe, tabs: tabsRecipe, menu: menuRecipe, inspector: inspectorRecipe },
  },
})
