import { mount } from "svelte";
import { reportViewport } from "../lib/viewport";
import "../styles/tokens.css";
import "../styles/macos.css";
import "../styles/windows.css";
import Settings from "../routes/settings/+page.svelte";

const app = mount(Settings, { target: document.getElementById("app")! });
reportViewport();
export default app;
