import { defineRecipe } from "@pandacss/dev"
import { focusRingStyles } from "./styles"

export const iconButtonRecipe = defineRecipe({
  className: "icon-button",
  description: "アイコンだけの、行の中に置く小さいボタン。タブを閉じる。指したときは塗りの地にする",
  base: {
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    flexShrink: "0",
    width: "compact",
    height: "compact",
    borderRadius: "full",
    color: "fg.muted",
    cursor: "default",
    outline: "none",
    transitionProperty: "background-color, opacity",
    transitionDuration: "fast",
    transitionTimingFunction: "standard",
    _hover: { bg: "fill", color: "fg" },
    _focusVisible: focusRingStyles,
    "& svg": { width: "mark", height: "mark" },
    "@media (prefers-reduced-motion: reduce)": { transitionDuration: "0s" },
  },
})
