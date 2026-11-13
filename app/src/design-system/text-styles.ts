import { defineTextStyles } from "@pandacss/dev"

export const textStyles = defineTextStyles({
  display: {
    value: {
      fontFamily: "display",
      fontSize: "display",
      fontWeight: "bold",
      lineHeight: "display",
    },
  },
  heading: {
    value: {
      fontFamily: "display",
      fontSize: "heading",
      fontWeight: "bold",
      lineHeight: "heading",
    },
  },
  ui: {
    value: {
      fontFamily: "text",
      fontSize: "ui",
      fontWeight: "regular",
      lineHeight: "ui",
    },
  },
  reading: {
    value: {
      fontFamily: "text",
      fontSize: "reading",
      fontWeight: "regular",
      lineHeight: "reading",
    },
  },
  mono: {
    value: {
      fontFamily: "mono",
      fontSize: "mono",
      fontWeight: "regular",
      lineHeight: "ui",
    },
  },
})
