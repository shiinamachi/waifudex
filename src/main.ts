import { mount } from "svelte";

import App from "./App.svelte";

const target = document.getElementById("app");

if (!target) {
  throw new Error("settings app mount target was not found");
}

mount(App, { target });
