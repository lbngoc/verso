import { mount } from "svelte";
import { reportViewport } from "../lib/viewport";
import "../styles/tokens.css";
import "../styles/macos.css";
import "../styles/windows.css";
import Update from "../routes/update/+page.svelte";

const app = mount(Update, { target: document.getElementById("app")! });
reportViewport();
export default app;
