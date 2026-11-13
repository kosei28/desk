import { render } from "@solidjs/web"
import { App } from "./app"
import "./index.css"

const darkScheme = window.matchMedia("(prefers-color-scheme: dark)")
const applyScheme = () => document.documentElement.classList.toggle("dark", darkScheme.matches)
applyScheme()
darkScheme.addEventListener("change", applyScheme)

render(() => <App />, document.getElementById("root")!)
