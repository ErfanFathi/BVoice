import { mount } from "svelte";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";
import App from "./App.svelte";
import Overlay from "./Overlay.svelte";

const label = getCurrentWindow().label;

const component = label === "overlay" ? Overlay : App;
if (label === "overlay") {
  document.body.dataset.overlay = "true";
}

export default mount(component, { target: document.getElementById("app")! });
