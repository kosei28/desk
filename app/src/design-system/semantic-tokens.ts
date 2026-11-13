import { defineSemanticTokens } from "@pandacss/dev"

const pair = (light: string, dark: string) => ({
  value: { base: `{colors.${light}}`, _dark: `{colors.${dark}}` },
})

export const semanticTokens = defineSemanticTokens({
  colors: {
    // 机。ウィンドウ全体に敷き、Liquid Glass が 4 割透ける
    ground: {
      value: {
        base: "color-mix(in srgb, {colors.violet.94} {opacity.ground}, transparent)",
        _dark: "color-mix(in srgb, {colors.ink.20} {opacity.ground}, transparent)",
      },
    },
    // 紙。いま読んでいるものだけが載る面。机に重ねて、Liquid Glass が机の 3 分の 1 ほど透ける
    paper: {
      DEFAULT: {
        value: {
          base: "color-mix(in srgb, {colors.ink.99} {opacity.paper}, transparent)",
          _dark: "color-mix(in srgb, {colors.ink.29} {opacity.paper}, transparent)",
        },
      },
      // 紙をさらに薄くしたもの（不透明度は紙 × hover）。dark の class は html に付くので、:root で解いた紙の色が明暗に合う
      hover: { value: "color-mix(in srgb, {colors.paper} {opacity.hover}, transparent)" },
    },
    fg: {
      DEFAULT: pair("ink.25", "ink.96"),
      muted: pair("ink.45", "ink.76"),
    },
    accent: pair("violet.50", "violet.78"),
    // status は 1 色につき 2 段。DEFAULT は印と線に使う濃い色、tint は面に使うパステル。needs_human は色を持たない（fg.muted）
    status: {
      backlog: { DEFAULT: pair("ink.60", "ink.80"), tint: pair("ink.94", "ink.34") },
      todo: { DEFAULT: pair("sky.60", "sky.76"), tint: pair("sky.94", "sky.34") },
      inprogress: { DEFAULT: pair("amber.71", "amber.84"), tint: pair("amber.94", "amber.34") },
      done: { DEFAULT: pair("leaf.60", "leaf.72"), tint: pair("leaf.94", "leaf.34") },
      dropped: { DEFAULT: pair("ink.60", "ink.80"), tint: pair("ink.94", "ink.34") },
    },
  },
  shadows: {
    paper: {
      value: {
        base: "0 1px 2px rgb(34 30 51 / 6%), 0 12px 32px -12px rgb(34 30 51 / 18%)",
        _dark: "0 1px 2px rgb(0 0 0 / 30%), 0 12px 32px -12px rgb(0 0 0 / 50%)",
      },
    },
  },
})
