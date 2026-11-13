import { defineConfig } from "@pandacss/dev"
import { deskPreset } from "./src/design-system/preset"

export default defineConfig({
  presets: ["@pandacss/preset-base", deskPreset],
  preflight: true,
  strictTokens: true,
  include: ["./src/**/*.{ts,tsx}"],
  exclude: [],
  outdir: "styled-system",
})
