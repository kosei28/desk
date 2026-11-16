import { defineSemanticTokens } from "@pandacss/dev"

const pair = (light: string, dark: string) => ({
  value: { base: `{colors.${light}}`, _dark: `{colors.${dark}}` },
})

// 文字の色を薄く重ねた色。置いた面の色を問わず、同じ差で見える。dark の class は html に付くので、:root で解いた fg が明暗に合う
const fgAt = (opacity: string) => ({ value: `color-mix(in srgb, {colors.fg} {opacity.${opacity}}, transparent)` })

// 読む面と同じだけ透かした色。ステータスの札の地
const statusPill = (light: string, dark: string) => ({
  value: {
    base: `color-mix(in srgb, ${light} {opacity.surface}, transparent)`,
    _dark: `color-mix(in srgb, ${dark} {opacity.surface}, transparent)`,
  },
})

export const semanticTokens = defineSemanticTokens({
  colors: {
    // 背景。ウィンドウ全体に敷き、Liquid Glass が 4 割透ける。左の列・タブの並び・右の列はこの上に載る
    background: {
      value: {
        base: "color-mix(in srgb, {colors.violet.94} {opacity.background}, transparent)",
        _dark: "color-mix(in srgb, {colors.ink.20} {opacity.background}, transparent)",
      },
    },
    // 読む面。真ん中の、いま開いているファイルを載せる面。背景に重ねて、壁紙が背景の 3 分の 1 ほど透ける
    surface: {
      value: {
        base: "color-mix(in srgb, {colors.ink.99} {opacity.surface}, transparent)",
        _dark: "color-mix(in srgb, {colors.ink.29} {opacity.surface}, transparent)",
      },
    },
    // 選んでいるタブと、左の列の選んでいる行の地。読む面と同じ（色も透け方も）で、影 raised と組んで背景から少し浮かせる。
    // 後ろが白に近いライトや、後ろが暗いダークでは地の差がほとんど無く、raised の縁の線と影で分ける
    selected: {
      value: {
        base: "color-mix(in srgb, {colors.ink.99} {opacity.surface}, transparent)",
        _dark: "color-mix(in srgb, {colors.ink.29} {opacity.surface}, transparent)",
      },
    },
    // 右の列の札（人が要る・タグ）の地。読む面と同じ地だが、影を付けず、選んでいる印には使わない（selected と役を分ける）
    pill: {
      value: {
        base: "color-mix(in srgb, {colors.ink.99} {opacity.surface}, transparent)",
        _dark: "color-mix(in srgb, {colors.ink.29} {opacity.surface}, transparent)",
      },
    },
    // 浮かぶもの（右クリックのメニュー）の面。下にある文字に重なっても読めるよう、ほぼ不透明にする
    float: {
      value: {
        base: "color-mix(in srgb, {colors.ink.99} {opacity.float}, transparent)",
        _dark: "color-mix(in srgb, {colors.ink.29} {opacity.float}, transparent)",
      },
    },
    fg: {
      DEFAULT: pair("ink.25", "ink.96"),
      // 読む面と浮かぶものの上の、控えめな文字（済んだ手順・引用・メニューのまとまりの名前）
      muted: pair("ink.45", "ink.76"),
      // 背景の上の、控えめな文字と印（開閉の矢印、人の印、右の列の場所の添え書き）。背景は壁紙が透けて中くらいの明るさにも
      // なるので、muted では読めなくなる。fg との差は 1 段だけ残し、壁紙によらず読めるほうを取る
      label: pair("ink.34", "ink.90"),
      // 背景の上の、右の列の項目の名前（頭・コードの名前と、見出しの中の名前）。label より 1 段薄く、隣の値の補助にする
      name: pair("ink.42", "ink.80"),
    },
    accent: pair("violet.50", "violet.78"),
    // 塗り。指した行・メニューの指した項目・本文のコードの地。背景の上でも読む面の上でも使う
    fill: fgAt("fill"),
    // 区切りの線。タブの間・メニューのまとまりの間・本文の表の行・区切り（hr）・引用
    separator: fgAt("separator"),
    // status の印（左の列・タブ・右の列のファイルの行）。パステルっぽく、明るく鮮やかさを抑えた色。needs_human は色を持たない（fg.label）
    status: {
      backlog: pair("ink.70", "ink.82"),
      todo: pair("sky.70", "sky.82"),
      inprogress: pair("amber.72", "amber.83"),
      done: pair("leaf.70", "leaf.82"),
      dropped: pair("red.67", "red.78"),
    },
    // 右の列のステータスの札。地は status の色相の色を読む面と同じだけ透かし、透ける分だけ元の色を濃くする。印と文字は白
    statusPill: {
      // ライトの灰の地（明るさ 60・彩度 0.036）は墨の段に無い色なので、ここにだけ置く
      backlog: statusPill("#807D95", "{colors.ink.74}"),
      todo: statusPill("{colors.sky.60}", "{colors.sky.74}"),
      inprogress: statusPill("{colors.amber.63}", "{colors.amber.74}"),
      done: statusPill("{colors.leaf.60}", "{colors.leaf.74}"),
      dropped: statusPill("{colors.red.56}", "{colors.red.70}"),
      fg: { value: "{colors.ink.99}" },
    },
  },
  shadows: {
    // 読む面の影。縁の線（1px）と、下に落ちる薄い影
    surface: {
      value: {
        base: "0 0 0 1px rgb(34 30 51 / 6%), 0 1px 2px rgb(34 30 51 / 6%), 0 12px 32px -12px rgb(34 30 51 / 18%)",
        _dark: "0 0 0 1px rgb(255 255 255 / 7%), 0 1px 2px rgb(0 0 0 / 30%), 0 12px 32px -12px rgb(0 0 0 / 50%)",
      },
    },
    // 浮かぶものの影。読む面より高く浮かせる
    float: {
      value: {
        base: "0 0 0 1px rgb(34 30 51 / 10%), 0 2px 6px rgb(34 30 51 / 8%), 0 16px 40px -8px rgb(34 30 51 / 24%)",
        _dark: "0 0 0 1px rgb(255 255 255 / 10%), 0 2px 6px rgb(0 0 0 / 35%), 0 16px 40px -8px rgb(0 0 0 / 60%)",
      },
    },
    // 選んでいるタブと左の列の選んでいる行。読む面と同じ色の小さい面を、背景からわずかに浮かせる
    raised: {
      value: {
        base: "0 0 0 1px rgb(34 30 51 / 5%), 0 1px 2px rgb(34 30 51 / 8%)",
        _dark: "0 0 0 1px rgb(255 255 255 / 6%), 0 1px 2px rgb(0 0 0 / 30%)",
      },
    },
  },
})
