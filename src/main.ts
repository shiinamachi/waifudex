import "./app.css";
import { mount } from "svelte";

import App from "./App.svelte";
import { createRuntimeStore } from "./lib/stores/runtimeStore.svelte";
import {
  getRuntimeBootstrap,
  listenRuntimeEvent,
  listenRuntimeSnapshot,
} from "./lib/tauri/runtimeTransport";

const app = document.getElementById("app");

const runtimeStore = createRuntimeStore({
  getRuntimeBootstrap,
  listenRuntimeSnapshot,
  listenRuntimeEvent,
});

const runtimeReady =
  app === null
    ? Promise.resolve()
    : runtimeStore.start().catch((error) => {
        console.error("failed to start runtime store", error);
      });

const appInstance =
  app === null
    ? null
    : mount(App, {
        target: app,
        props: {
          store: runtimeStore,
          runtimeReady,
        },
      });

export { runtimeStore };
export default appInstance;
