import { mount } from "svelte";
import "../styles/tokens.css";
import "../styles/macos.css";
import "../styles/windows.css";
import Popup from "../routes/popup/+page.svelte";

const app = mount(Popup, { target: document.getElementById("app")! });
export default app;
