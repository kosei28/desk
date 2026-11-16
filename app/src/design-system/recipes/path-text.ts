import { defineRecipe } from "@pandacss/dev"

export const pathTextRecipe = defineRecipe({
  className: "path-text",
  description: "パス。等幅で、折り返すときは / の後ろで折り、ハイフンの後ろでは折らない。色は置いた面に合わせて呼ぶ側が決める",
  base: {
    textStyle: "mono",
    // / の後ろで折れない長い名前は、どこでも折る
    overflowWrap: "anywhere",
    // ハイフンとその次の 1 字。ハイフンの後ろでは折らない
    "& [data-keep]": { whiteSpace: "nowrap" },
  },
})
