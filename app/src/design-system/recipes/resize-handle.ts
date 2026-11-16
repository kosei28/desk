import { defineRecipe } from "@pandacss/dev"

export const resizeHandleRecipe = defineRecipe({
  className: "resize-handle",
  description:
    "UI SDK の ResizableHandle。列の間の 8px の余白で、線は引かない。引いているときはカーソルだけが変わり、キーで動かしているときだけフォーカスの輪の色のまるい印を出す",
  base: {
    position: "relative",
    width: "token(spacing.2)",
    outline: "none",
    cursor: "col-resize",
    "&::after": {
      content: '""',
      position: "absolute",
      insetInlineStart: "50%",
      top: "50%",
      width: "token(borderWidths.focus)",
      height: "token(spacing.8)",
      borderRadius: "full",
      bg: "accent",
      opacity: "0",
      transform: "translate(-50%, -50%)",
      transitionProperty: "opacity",
      transitionDuration: "fast",
      transitionTimingFunction: "standard",
    },
    "&:focus-visible::after": { opacity: "1" },
    "@media (prefers-reduced-motion: reduce)": { "&::after": { transitionDuration: "0s" } },
  },
})
