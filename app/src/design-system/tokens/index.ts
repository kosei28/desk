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
      "34": { value: "#383546" },
      // 右の列の項目の名前（ライト）。34 と 45 の間に足した段
      "42": { value: "#4D4A5F" },
      "45": { value: "#555268" },
      // 灰の status。70 はライトの印、74 はダークの札の地、82 はダークの印
      "70": { value: "#9E9BB4" },
      "74": { value: "#AAA7C0" },
      "76": { value: "#B1AEC3" },
      "80": { value: "#BEBBD0" },
      "82": { value: "#C4C1D9" },
      "90": { value: "#DEDBF1" },
      "96": { value: "#F2F1F8" },
      "99": { value: "#FDFDFF" },
    },
    violet: {
      "50": { value: "#6B40C9" },
      "78": { value: "#B9A9FF" },
      "94": { value: "#E9E6FE" },
    },
    // status の 4 色相。空（250°）は待つ、琥珀（70°）は動いている、若葉（148°）は済んだ、赤（25°）はやめた。琥珀と若葉は、
    // すみれ（292°）の補色（112°）を挟む両側。明るさはそろえ、色相と印の形で分ける。鮮やかさは並べて見てそろうように色ごとに決め、
    // どれも sRGB の端までの 9 割以下にする。
    // 段の使い分け: ライトの印 70（琥珀 72・赤 67）、ダークの印 82（琥珀 83・赤 78）。ステータスの札の地は 65% で透かす前提で
    // 濃く鮮やかにし、ライト 60（琥珀 63・赤 56）、ダーク 74（赤 70）
    sky: {
      "60": { value: "#2884D5" },
      "70": { value: "#6DA3DA" },
      "74": { value: "#6BB0F5" },
      "82": { value: "#9CC9F8" },
    },
    amber: {
      "63": { value: "#B77A29" },
      "72": { value: "#CD995C" },
      "74": { value: "#DF9B44" },
      "83": { value: "#EEBD86" },
    },
    leaf: {
      "60": { value: "#2E9848" },
      "70": { value: "#72B07B" },
      "74": { value: "#67C377" },
      "82": { value: "#9CD5A3" },
    },
    red: {
      "56": { value: "#C8393A" },
      "67": { value: "#D5766F" },
      "70": { value: "#F56E67" },
      "78": { value: "#F19E97" },
    },
  },
  opacity: {
    // 背景と読む面の不透明度は、壁紙の透け方を画面で見て決める。後ろが真っ黒でも真っ白でも、読む面の文字（fg・muted・accent）が
    // 4.5:1 前後に収まることを、読めなくなる極端を避ける目安として確かめる
    background: { value: "62%" },
    surface: { value: "65%" },
    // 浮かぶもの（右クリックのメニュー）。下の文字に重なっても読めるよう、ほぼ不透明にして後ろはぼかす
    float: { value: "92%" },
    // 塗りと区切りの線は、文字の色を薄く重ねて作る。背景の上でも読む面の上でも、同じ差で見える
    fill: { value: "8%" },
    separator: { value: "12%" },
  },
  fonts: {
    text: { value: 'system-ui, "Hiragino Sans", sans-serif' },
    // Zen Maru Gothic は unicode-range で和文だけを持つ（index.css）。ui-rounded より前に置く。WebKit の ui-rounded は
    // OS の書体の列ごと来て、和文をヒラギノ角ゴで拾うので、後ろに置くと和文が丸にならない
    display: { value: '"Zen Maru Gothic", ui-rounded, system-ui, sans-serif' },
    mono: { value: 'ui-monospace, "SF Mono", Menlo, monospace' },
  },
  // 大きさは役割に割り当てる（デザイン言語の 5 節）。ラベルとパスは 12px、操作と一覧は 13px、本文は 16px、見出しは 16・20・28px
  fontSizes: {
    label: { value: "12px" },
    mono: { value: "12px" },
    ui: { value: "13px" },
    reading: { value: "16px" },
    subheading: { value: "16px" },
    heading: { value: "20px" },
    title: { value: "28px" },
  },
  fontWeights: {
    regular: { value: "400" },
    bold: { value: "700" },
  },
  lineHeights: {
    title: { value: "1.3" },
    heading: { value: "1.4" },
    label: { value: "1.4" },
    ui: { value: "1.5" },
    reading: { value: "1.7" },
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
    // 左の列の字下げ。行の頭の枠（16px）と、枠の後ろの間（6px）の和。子の行の枠が、親の行の文字の頭に来る
    treeIndent: { value: "22px" },
  },
  sizes: {
    "0": { value: "0" },
    full: { value: "100%" },
    mark: { value: "12px" },
    icon: { value: "16px" },
    // メニュー・タブ・右の列の 1 行。13px の文字に上下 7px
    row: { value: "28px" },
    // 左の列の 1 行。前のタスク（desk-app-stack）で作った形の高さ
    treeRow: { value: "30px" },
    // 右の列の札。12px の文字と 12px の印を入れる
    pill: { value: "22px" },
    // 行の中に置く小さいボタン（タブを閉じる）
    compact: { value: "20px" },
    // タブの幅の上限。13px で 13 字ほどの見出しに、印と閉じるボタンを足して収まる。収まらない見出しは末尾を省く
    tab: { value: "240px" },
    // タブの幅の下限。開いているタブが帯に収まらないときは、ここまで縮めてから横に送る。見出しが 4 字ほど見える
    tabMin: { value: "120px" },
    // メニューの幅の下限。項目の名前が短くても、幅がそろって見えるように
    menu: { value: "200px" },
    // 信号機の帯の高さ。信号機（14px）は左と上から 19px（tauri.conf.json の trafficLightPosition）で、
    // 下にも同じ 19px を取って帯の縦の中央に置く（19 + 14 + 19）
    titlebar: { value: "52px" },
    // 左の列と右の列は同じ幅（列の間の 8px を含む）
    sidebar: { value: "300px" },
    inspector: { value: "300px" },
    measure: { value: "40em" },
  },
  radii: {
    // 本文の中のコード。行の高さの中に収まる小さい角
    inline: { value: "4px" },
    // ウィンドウの角（16px）と同心。読む面はウィンドウの縁から 8px 内側にあるので 16 - 8
    surface: { value: "8px" },
    // 浮かぶもの（メニュー）と、その中の行。行は浮かぶものの内側 4px にあるので 14 - 4
    float: { value: "14px" },
    item: { value: "10px" },
    // 右の列のファイルの行。行の高さ（sizes.row の 28px）の半分なので、1 行ならまるく、折り返すと角の丸い四角になる
    row: { value: "14px" },
    full: { value: "9999px" },
  },
  borderWidths: {
    // 区切りの線。表の行・区切り（hr）・引用・浮かぶものの縁
    hairline: { value: "1px" },
    mark: { value: "1.5px" },
    focus: { value: "2px" },
    // 本文の引用の左の線
    quote: { value: "3px" },
  },
  // 浮かぶもの（右クリックのメニュー）の後ろのぼかし
  blurs: {
    float: { value: "24px" },
  },
  durations: {
    fast: { value: "120ms" },
    base: { value: "200ms" },
  },
  easings: {
    standard: { value: "cubic-bezier(0.2, 0, 0, 1)" },
  },
  // 重なり順。帯（窓を動かす所）はウィンドウの幅いっぱいの層で、タブと読む面はその上に載る
  zIndex: {
    titlebar: { value: 0 },
    surface: { value: 1 },
  },
})
