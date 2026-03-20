import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import App from "./App";

const target = document.getElementById("app");

if (!target) {
  throw new Error("settings app mount target was not found");
}

createRoot(target).render(
  <StrictMode>
    <App />
  </StrictMode>
);
