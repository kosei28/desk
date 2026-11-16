import { defineRecipe } from "@pandacss/dev"

export const surfaceRecipe = defineRecipe({
  className: "surface",
  description:
    "読む面。真ん中の、いま開いているファイルを載せる面。背景より透けが少なく、縁の線と影で背景から浮かせる。中身は本文の幅（measure）の 1 列で、面の中央に置く",
  // 余白の決め方はデザイン言語の 8 節。内側は四方とも 48px から始め、幅が余ったら左右に同じだけ分ける。
  // 48px は本文の中のいちばん広い間（h2 の上の 40px）より 1 段広い値で、面の縁が本文の節の区切りより強く見える
  base: {
    bg: "surface",
    color: "fg",
    // 本文の幅（40em）を本文の文字の大きさで測る
    textStyle: "reading",
    borderRadius: "surface",
    boxShadow: "surface",
    overflow: "auto",
    display: "grid",
    gridTemplateColumns: "minmax(0, token(sizes.measure))",
    justifyContent: "center",
    alignContent: "start",
    padding: "12",
  },
})
