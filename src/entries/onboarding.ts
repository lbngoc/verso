import { mount } from "svelte";
import "../styles/tokens.css";
import "../styles/macos.css";
import "../styles/windows.css";
import Onboarding from "../routes/onboarding/+page.svelte";

const app = mount(Onboarding, { target: document.getElementById("app")! });
export default app;
