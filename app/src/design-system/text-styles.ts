import { defineTextStyles } from "@pandacss/dev"

// 文字は役割で選ぶ（デザイン言語の 5 節）。大きさだけを変えず、書体・太さ・行の高さを 1 組にして使う
export const textStyles = defineTextStyles({
  // 読む面のいちばん上の、ファイルの見出し（h1）
  title: {
    value: {
      fontFamily: "display",
      fontSize: "title",
      fontWeight: "bold",
      lineHeight: "title",
      fontFeatureSettings: '"palt"',
    },
  },
  // 本文の h2
  heading: {
    value: {
      fontFamily: "display",
      fontSize: "heading",
      fontWeight: "bold",
      lineHeight: "heading",
      fontFeatureSettings: '"palt"',
    },
  },
  // 本文の h3
  subheading: {
    value: {
      fontFamily: "display",
      fontSize: "subheading",
      fontWeight: "bold",
      lineHeight: "heading",
    },
  },
  // 本文
  reading: {
    value: {
      fontFamily: "text",
      fontSize: "reading",
      fontWeight: "regular",
      lineHeight: "reading",
    },
  },
  // 操作と一覧。左の列の行・タブ・右の列の値・メニュー
  ui: {
    value: {
      fontFamily: "text",
      fontSize: "ui",
      fontWeight: "regular",
      lineHeight: "ui",
    },
  },
  // ラベル。右の列の項目の名前と札の文字。背景の上の名前は fg.name、読む面の上では fg.muted と組む
  label: {
    value: {
      fontFamily: "text",
      fontSize: "label",
      fontWeight: "regular",
      lineHeight: "label",
    },
  },
  // 右の列の見出し（コード・親子と依存・関連ファイル・リンク・他からのリンク）。色は fg のまま（背景の上で薄くすると読めない）
  group: {
    value: {
      fontFamily: "text",
      fontSize: "label",
      fontWeight: "bold",
      lineHeight: "label",
    },
  },
  // パスとコード
  mono: {
    value: {
      fontFamily: "mono",
      fontSize: "mono",
      fontWeight: "regular",
      lineHeight: "ui",
    },
  },
})
