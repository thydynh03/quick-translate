export const IS_MAC = typeof navigator !== "undefined" && (
  /Mac|iPhone|iPod|iPad/i.test(navigator.userAgent) ||
  (typeof navigator.platform === "string" && navigator.platform.includes("Mac"))
);

/**
 * Builds a shortcut string such as "Option+Q" (macOS) or "Alt+Q" (Windows) from a keyboard event.
 */
export function shortcutFrom(event: React.KeyboardEvent): string | null {
  const key = event.key;
  if (["Control", "Shift", "Alt", "Meta"].includes(key)) return null;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push(IS_MAC ? "Control" : "Ctrl");
  if (event.altKey) parts.push(IS_MAC ? "Option" : "Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push(IS_MAC ? "Cmd" : "Super");
  if (parts.length === 0) return null;
  const code = event.code;
  let name: string;
  if (code.startsWith("Key")) name = code.slice(3);
  else if (code.startsWith("Digit")) name = code.slice(5);
  else if (/^F\d{1,2}$/.test(code)) name = code;
  else if (code === "Space") name = "Space";
  else return null;
  return [...parts, name].join("+");
}

/**
 * Formats a shortcut string for display.
 * On macOS:
 *  - "Alt" / "Option" -> "⌥ Option"
 *  - "Super" / "Cmd" / "Command" -> "⌘ Cmd"
 *  - "Control" / "Ctrl" -> "⌃ Control"
 *  - "Shift" -> "⇧ Shift"
 *  - Displayed as: "⌥ Option + Q"
 * On Windows:
 *  - Displayed as: "Alt+Q", "Ctrl+Shift+V", etc.
 */
export function formatHotkey(shortcut: string | null | undefined, isMac: boolean = IS_MAC): string {
  if (!shortcut || !shortcut.trim()) return "";
  if (isMac) {
    return shortcut
      .split("+")
      .map((part) => {
        const trimmed = part.trim();
        switch (trimmed) {
          case "Alt":
          case "Option":
            return "⌥ Option";
          case "Super":
          case "Cmd":
          case "Command":
          case "Meta":
            return "⌘ Cmd";
          case "Control":
          case "Ctrl":
            return "⌃ Control";
          case "Shift":
            return "⇧ Shift";
          default:
            return trimmed;
        }
      })
      .join(" + ");
  }
  return shortcut;
}
