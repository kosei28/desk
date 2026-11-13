import { defineRecipe } from "@pandacss/dev"

export const statusMarkRecipe = defineRecipe({
  className: "status-mark",
  description: "status の印。色で見分け、円の塗り方で進み具合を補う",
  // status は実行時に決まるので、variant を全部出しておく
  staticCss: ["*"],
  base: {
    flexShrink: "0",
    width: "mark",
    height: "mark",
  },
  variants: {
    status: {
      backlog: { color: "status.backlog" },
      todo: { color: "status.todo" },
      inprogress: { color: "status.inprogress" },
      done: { color: "status.done" },
      dropped: { color: "status.dropped" },
    },
  },
})
