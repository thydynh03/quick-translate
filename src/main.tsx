import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import Capture from "./Capture";
import Clipboard from "./Clipboard";
import Popup from "./Popup";
import Settings from "./Settings";
import "./styles.css";
import type { Settings as Config } from "./types";

// One bundle, four windows: the popup, the OCR capture overlay, the clipboard board and settings.
const label = getCurrentWebviewWindow().label;
document.documentElement.dataset.window = label;

// Light or dark from the settings; "system" leaves it to the system.
const applyTheme = () =>
  invoke<Config>("get_settings").then(({ theme }) => {
    const chosen = theme === "light" || theme === "dark" ? theme : null;
    if (chosen) document.documentElement.dataset.theme = chosen;
    else delete document.documentElement.dataset.theme;
    // The title bar of the settings window as well.
    if (label === "main") getCurrentWebviewWindow().setTheme(chosen).catch(() => {});
  });
applyTheme();
listen("settings-changed", applyTheme);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{label === "popup" ? <Popup /> : label === "capture" ? <Capture /> : label === "clipboard" ? <Clipboard /> : <Settings />}</React.StrictMode>,
);
