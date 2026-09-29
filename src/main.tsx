import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";

const loadApp =
  getCurrentWindow().label === "reminder"
    ? () => import("./ReminderApp")
    : () => import("./App");
const appElement = React.createElement(React.lazy(loadApp));

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <React.Suspense fallback={null}>{appElement}</React.Suspense>
  </React.StrictMode>,
);
