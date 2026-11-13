import { defineTokens } from "@pandacss/dev"

// 色の名前の数字は OKLCH の明るさ（L × 100）。数字が離れているほど、重ねたときのコントラストが強い。
// 値は画面に並べて見て決める。コントラスト比や色差の数値は、見た目を裏づけ、極端に悪いものを避けるための目安
export const tokens = defineTokens({
  colors: {
    transparent: { value: "transparent" },
    // すみれの色相（292°）に寄せた墨。彩度は 0.04 まで
    ink: {
      "20": { value: "#161325" },
      "25": { value: "#221E33" },
      "29": { value: "#2A2738" },
      "34": { value: "#38373E" },
      "45": { value: "#555268" },
      "60": { value: "#807E8A" },
      "76": { value: "#B1AEC3" },
      "80": { value: "#BEBBD0" },
      "94": { value: "#EBEAF3" },
      "96": { value: "#F2F1F8" },
      "99": { value: "#FDFDFF" },
    },
    violet: {
      "50": { value: "#6B40C9" },
      "78": { value: "#B9A9FF" },
      "94": { value: "#E9E6FE" },
    },
    // status の 3 色。空（250°）は待つ、琥珀（70°）は動いている、若葉（148°）は済んだ。琥珀と若葉は、すみれ（292°）の
    // 補色（112°）を挟む両側。印の段は、琥珀を明るく、空と若葉を一段暗くして、色相が見分けにくい目にも明るさで分ける。
    // どれも sRGB の端から 1 割以上離す。面の段（94・34）は彩度 0.03・0.045 まで
    sky: {
      "34": { value: "#263A4E" },
      "60": { value: "#3284D0" },
      "76": { value: "#79B6F4" },
      "94": { value: "#E1EFFD" },
    },
    amber: {
      "34": { value: "#47341D" },
      "71": { value: "#D8902D" },
      "84": { value: "#FABD75" },
      "94": { value: "#FAEAD8" },
    },
    leaf: {
      "34": { value: "#273E2B" },
      "60": { value: "#409551" },
      "72": { value: "#67BB75" },
      "94": { value: "#E0F3E2" },
    },
  },
  opacity: {
    // 地と紙の不透明度は、壁紙の透け方を画面で見て決める。後ろが真っ黒でも真っ白でも、上の文字（fg・muted・accent）が
    // 4.5:1 前後に収まることを、読めなくなる極端を避ける目安として確かめる
    ground: { value: "62%" },
    paper: { value: "65%" },
    // 置いた面の不透明度に掛ける。紙のホバーは紙の 55%
    hover: { value: "55%" },
  },
  fonts: {
    text: { value: 'system-ui, "Hiragino Sans", sans-serif' },
    display: { value: 'ui-rounded, "Zen Maru Gothic", system-ui, sans-serif' },
    mono: { value: 'ui-monospace, "SF Mono", Menlo, monospace' },
  },
  fontSizes: {
    mono: { value: "12px" },
    ui: { value: "13px" },
    reading: { value: "16px" },
    heading: { value: "20px" },
    display: { value: "28px" },
  },
  fontWeights: {
    regular: { value: "400" },
    bold: { value: "700" },
  },
  lineHeights: {
    display: { value: "1.3" },
    heading: { value: "1.4" },
    ui: { value: "1.5" },
    reading: { value: "1.85" },
  },
  spacing: {
    "0": { value: "0" },
    "1": { value: "4px" },
    "1.5": { value: "6px" },
    "2": { value: "8px" },
    "3": { value: "12px" },
    "4": { value: "16px" },
    "6": { value: "24px" },
    "8": { value: "32px" },
    "10": { value: "40px" },
    "12": { value: "48px" },
    // ツリーの字下げは、行の頭の枠（16px）と間（6px）の和。子の行の頭が、親の行の文字の頭にそろう
    treeIndent: { value: "22px" },
  },
  sizes: {
    "0": { value: "0" },
    full: { value: "100%" },
    mark: { value: "12px" },
    icon: { value: "16px" },
    row: { value: "30px" },
    // 信号機の帯の高さ。信号機（14px）は左と上から 19px（tauri.conf.json の trafficLightPosition）で、
    // 下にも同じ 19px を取って帯の縦の中央に置く（19 + 14 + 19）
    titlebar: { value: "52px" },
    sidebar: { value: "296px" },
    measure: { value: "40em" },
  },
  radii: {
    // ウィンドウの角（16px）と同心。紙はウィンドウの縁から 8px 内側にあるので 16 - 8
    sheet: { value: "8px" },
    full: { value: "9999px" },
  },
  borderWidths: {
    mark: { value: "1.5px" },
    focus: { value: "2px" },
  },
  durations: {
    fast: { value: "120ms" },
    base: { value: "200ms" },
  },
  easings: {
    standard: { value: "cubic-bezier(0.2, 0, 0, 1)" },
  },
  // 重なり順。帯（窓を動かす所）はウィンドウの幅いっぱいの層で、紙はその上に載る
  zIndex: {
    titlebar: { value: 0 },
    paper: { value: 1 },
  },
})
