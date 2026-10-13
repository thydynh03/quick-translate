import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import Capture from "./Capture";
import Clipboard from "./Clipboard";
import Popup from "./Popup";
import Settings from "./Settings";
import "./styles.css";

// One bundle, four windows: the popup, the OCR capture overlay, the clipboard board and settings.
const label = getCurrentWebviewWindow().label;
document.documentElement.dataset.window = label;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{label === "popup" ? <Popup /> : label === "capture" ? <Capture /> : label === "clipboard" ? <Clipboard /> : <Settings />}</React.StrictMode>,
);
