import React from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import "./styles.css";

const container = document.getElementById("root");
if (container === null) {
  throw new Error("#root が見つかりません。index.html と main.tsx の対応を確認してください。");
}

createRoot(container).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
