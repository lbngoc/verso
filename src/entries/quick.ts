import { mount } from "svelte";
import { reportViewport } from "../lib/viewport";
import "../styles/tokens.css";
import "../styles/macos.css";
import "../styles/windows.css";
import Quick from "../routes/quick/+page.svelte";

const app = mount(Quick, { target: document.getElementById("app")! });
reportViewport();
export default app;
