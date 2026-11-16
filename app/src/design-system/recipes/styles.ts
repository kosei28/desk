import { defineStyles } from "@pandacss/dev"

// 浮かぶもの（右クリックのメニュー）の面。ほぼ不透明な地に後ろのぼかしを足し、縁の線と深い影で読む面より上に浮かせる
export const floatStyles = defineStyles({
  bg: "float",
  color: "fg",
  boxShadow: "float",
  backdropFilter: "blur(token(blurs.float))",
  outline: "none",
  transitionProperty: "opacity",
  transitionDuration: "fast",
  transitionTimingFunction: "standard",
  "&[data-starting-style], &[data-ending-style]": { opacity: "0" },
  "@media (prefers-reduced-motion: reduce)": { transitionDuration: "0s" },
})

// キーで動かしたときの輪。内側に描く。外に出すと、間の無い隣の行の地に線が隠れ、スクロールの入れ物に端の線が切られる
export const focusRingStyles = defineStyles({
  outlineWidth: "focus",
  outlineStyle: "solid",
  outlineColor: "accent",
  outlineOffset: "calc(-1 * token(borderWidths.focus))",
})

// 行の地の動き。指したとき・選んだときに地が変わる行（左の列・タブ・メニュー）でそろえる
export const rowTransitionStyles = defineStyles({
  transitionProperty: "background-color, box-shadow, color",
  transitionDuration: "fast",
  transitionTimingFunction: "standard",
  "@media (prefers-reduced-motion: reduce)": { transitionDuration: "0s" },
})
