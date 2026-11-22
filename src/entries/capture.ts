import { mount } from "svelte";
import "../styles/tokens.css";
import "../styles/macos.css";
import "../styles/windows.css";
import Capture from "../routes/capture/+page.svelte";

const app = mount(Capture, { target: document.getElementById("app")! });
export default app;
