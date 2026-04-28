import { mount } from "svelte";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";
import App from "./App.svelte";
import Overlay from "./Overlay.svelte";

const component = getCurrentWindow().label === "overlay" ? Overlay : App;

export default mount(component, { target: document.getElementById("app")! });
