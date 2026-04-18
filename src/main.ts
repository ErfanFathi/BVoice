import { mount } from "svelte";
import "./App.css";
import App from "./App.svelte";

export default mount(App, { target: document.getElementById("app")! });
