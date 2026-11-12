import { mount } from "svelte";
import "@fontsource/ibm-plex-sans/400.css";
import "@fontsource/ibm-plex-sans/600.css";
import "@fontsource/cascadia-code/400.css";
import "./app.css";
import App from "./App.svelte";
import { applyTheme, theme } from "./lib/theme.svelte";

applyTheme(theme.value);

const app = mount(App, { target: document.getElementById("app")! });

export default app;
