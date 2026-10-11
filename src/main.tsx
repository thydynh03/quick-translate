import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import Capture from "./Capture";
import Popup from "./Popup";
import Settings from "./Settings";
import "./styles.css";

// One bundle, three windows: the popup, the OCR capture overlay and settings.
const label = getCurrentWebviewWindow().label;
document.documentElement.dataset.window = label;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{label === "popup" ? <Popup /> : label === "capture" ? <Capture /> : <Settings />}</React.StrictMode>,
);
