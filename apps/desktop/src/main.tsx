import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource-variable/space-grotesk";
import "./brand/tokens.css";
import "./brand/base.css";
import "./panels/chrome.css";
import "./panels/side.css";
import "./panels/floating.css";
import "./panels/bottom.css";
import "./layouts/layouts.css";
import "./prototype/prototype.css";
import { App } from "./App";

const root = document.getElementById("root");

if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
