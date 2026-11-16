import { defineRecipe, defineStyles } from "@pandacss/dev"

const linkBase = defineStyles({
  color: "fg",
  textDecorationColor: "accent",
  textDecorationThickness: "token(borderWidths.hairline)",
  textUnderlineOffset: "0.25em",
  cursor: "default",
  borderRadius: "inline",
  _focusVisible: {
    outlineWidth: "focus",
    outlineStyle: "solid",
    outlineColor: "accent",
    outlineOffset: "token(borderWidths.focus)",
  },
})

// 本文の中のリンク。文に混ざるので、いつも下線を引いて見分ける。文字は fg のまま、下線だけをアクセントにする
export const inlineLinkStyles = defineStyles({ ...linkBase, textDecorationLine: "underline" })

export const linkRecipe = defineRecipe({
  className: "link",
  description: "リンク。inline は本文の中（いつも下線）、quiet は右の列のようにリンクだけが並ぶ所（指したときだけ下線）",
  base: linkBase,
  variants: {
    tone: {
      inline: { textDecorationLine: "underline" },
      quiet: { textDecorationLine: "none", _hover: { textDecorationLine: "underline" } },
    },
  },
  defaultVariants: { tone: "inline" },
})
