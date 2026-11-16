import { defineSlotRecipe } from "@pandacss/dev"
import { focusRingStyles, rowTransitionStyles } from "./styles"

export const tabsRecipe = defineSlotRecipe({
  className: "tabs",
  description:
    "UI SDK の Tabs の、読む面の上の帯に並べるタブ。幅はどのタブも同じで、タブの間に縦の区切りの線を引く。選んでいるタブは左の列の選んでいる行と同じ地にする。帯に収まらないときは下限まで縮めてから横に送り、送れる側の端を背景へぼかして続きがあることを示す。閉じるボタンはタブ（button）の中に入れられないので、item の中でタブの隣に置き、タブの右端に重ねる",
  slots: ["area", "viewport", "list", "item", "tab", "title", "close"],
  base: {
    // UI SDK の ScrollArea。横に送れる量は根の変数（--scroll-area-overflow-x-start・-end）に出る
    area: {
      display: "flex",
      minWidth: "0",
    },
    // 送る所。横に送るときに、選んでいるタブの影とフォーカスの輪が切れないよう、内側に 4px 空けて外側で打ち消す。
    // 送れる側の端を、最大でいちばん狭いタブ 1 つ分（120px）かけて消し、帯がその先に続くことを示す。タブの右の 30px ほどは
    // 閉じるボタンの場所で何も無く、端でタブがちょうど収まることもあるので、短いぼかしでは見出しに届かず、並びが普通に
    // 終わって見える。1 つ分あれば、どこで切れても端のタブの見出しが薄れる。送れる量がそれより少ないときは、その量だけぼかす
    viewport: {
      flex: "1",
      minWidth: "0",
      padding: "1",
      margin: "-1",
      scrollbarWidth: "none",
      // 今のタブを見える所まで送るとき（scrollIntoView）、ぼかした端に置かない
      scrollPaddingInline: "tabMin",
      "--scroll-area-overflow-x-start": "inherit",
      "--scroll-area-overflow-x-end": "inherit",
      "--fade-start": "min(token(sizes.tabMin), var(--scroll-area-overflow-x-start, 0px))",
      "--fade-end": "min(token(sizes.tabMin), var(--scroll-area-overflow-x-end, 0px))",
      maskImage:
        "linear-gradient(to right, transparent, black var(--fade-start), black calc(100% - var(--fade-end)), transparent)",
    },
    list: {
      display: "flex",
      alignItems: "center",
      gap: "1",
      minWidth: "0",
    },
    item: {
      ...rowTransitionStyles,
      position: "relative",
      display: "flex",
      // 幅はどのタブも同じにして、上限（tab）と下限（tabMin）の間で帯に合わせる。見出しの長さで幅を変えると、
      // 帯に収まらないときに全部のタブが少しずつ違う所で省かれる
      flex: "1 1 0",
      minWidth: "tabMin",
      maxWidth: "tab",
      height: "row",
      borderRadius: "full",
      _hover: { bg: "fill" },
      "&:has([data-selected])": { bg: "selected", boxShadow: "raised" },
      "&[data-menu-open]": { bg: "fill" },
      // タブの間の区切り。地のあるタブ（選んでいる・指している・メニューを開いている）の両隣では消す
      "& + &::before": {
        content: '""',
        position: "absolute",
        insetInlineStart: "[calc(token(spacing.1) / -2 - 0.5px)]",
        top: "50%",
        width: "token(borderWidths.hairline)",
        height: "icon",
        translate: "0 -50%",
        bg: "separator",
      },
      "&:is(:hover, :has([data-selected]), [data-menu-open])::before, :is(:hover, :has([data-selected]), [data-menu-open]) + &::before":
        { opacity: "0" },
    },
    tab: {
      display: "flex",
      alignItems: "center",
      gap: "1.5",
      flex: "1",
      minWidth: "0",
      height: "full",
      paddingInlineStart: "3",
      // 閉じるボタンの場所を空けておく。ボタンが出ていないときも、見出しの省き方が変わらないようにする
      paddingInlineEnd: "calc(token(sizes.compact) + token(spacing.1) * 2)",
      borderRadius: "full",
      textStyle: "ui",
      color: "fg",
      cursor: "default",
      userSelect: "none",
      outline: "none",
      _focusVisible: focusRingStyles,
    },
    title: {
      minWidth: "0",
      overflow: "hidden",
      textOverflow: "ellipsis",
      whiteSpace: "nowrap",
    },
    // 選んでいるタブと、指したタブにだけ出す。ほかのタブで出すと、並びがボタンだらけに見える
    close: {
      position: "absolute",
      insetInlineEnd: "1",
      top: "50%",
      translate: "0 -50%",
      opacity: "0",
      "[data-slot=tabs-tab][data-selected] + &, :is(:hover, :focus-within) > &": { opacity: "1" },
    },
  },
})
