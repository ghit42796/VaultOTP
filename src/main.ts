import "./app.css";
import { applyTheme, loadThemePref } from "./lib/theme";
import App from "./App.svelte";

applyTheme(loadThemePref());

const app = new App({ target: document.getElementById("app")! });
export default app;
