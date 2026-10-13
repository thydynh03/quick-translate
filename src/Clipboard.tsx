import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Item = { id: number; text: string; pinned: boolean; app: string | null; at: number };

const MAC = navigator.userAgent.includes("Mac");
/** How the command key is written on this system. */
const MOD = MAC ? "⌘" : "Ctrl+";

/** A letter for what the text looks like: a link, code or plain text. */
function kind(text: string): string {
  const trimmed = text.trim();
  if (/^(https?:\/\/|www\.)\S+$/i.test(trimmed)) return "↗";
  if (/[{};]\s*$|=>|^\s*(const|let|import|def|fn|function|class|#include|\$ )/m.test(trimmed)) return "<>";
  return "Aa";
}

function ago(at: number): string {
  if (!at) return "";
  const minutes = Math.floor((Date.now() - at) / 60000);
  if (minutes < 1) return "vừa xong";
  if (minutes < 60) return `${minutes} phút trước`;
  if (minutes < 60 * 24) return `${Math.floor(minutes / 60)} giờ trước`;
  return `${Math.floor(minutes / 60 / 24)} ngày trước`;
}

/** The texts copied lately, laid out like Spotlight: pick one to paste it where you were typing. */
export default function Clipboard() {
  const [items, setItems] = useState<Item[]>([]);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const search = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);

  const load = () => invoke<Item[]>("clipboard_list").then(setItems);

  useEffect(() => {
    load();
    const opened = listen("clipboard-open", () => {
      setQuery("");
      setActive(0);
      load();
      search.current?.focus();
    });
    const changed = listen("clipboard-changed", load);
    return () => {
      opened.then((off) => off());
      changed.then((off) => off());
    };
  }, []);

  // The newest first, then the pinned ones.
  const [recent, pinned] = useMemo(() => {
    const wanted = query.trim().toLowerCase();
    const found = wanted ? items.filter((item) => item.text.toLowerCase().includes(wanted)) : items;
    return [found.filter((item) => !item.pinned), found.filter((item) => item.pinned)];
  }, [items, query]);
  const shown = useMemo(() => [...recent, ...pinned], [recent, pinned]);
  const index = Math.min(active, Math.max(shown.length - 1, 0));
  const current: Item | undefined = shown[index];

  useEffect(() => {
    list.current?.querySelector(".board-item.active")?.scrollIntoView({ block: "nearest" });
  }, [index, shown]);

  const paste = (item?: Item) => item && invoke("clipboard_paste", { id: item.id });
  const translate = (item?: Item) => item && invoke("clipboard_translate", { id: item.id });
  const pin = (item?: Item) => item && invoke("clipboard_pin", { id: item.id, pinned: !item.pinned });
  const remove = (item?: Item) => item && invoke("clipboard_remove", { id: item.id });

  const onKeyDown = (event: KeyboardEvent) => {
    const command = MAC ? event.metaKey : event.ctrlKey;
    // By position on the keyboard when known, so other layouts work; by character otherwise.
    const digit = /^Digit\d$/.test(event.code) ? event.code.slice(5) : event.key;
    const letter = /^Key[A-Z]$/.test(event.code) ? event.code.slice(3).toLowerCase() : event.key.toLowerCase();
    if (event.key === "Escape") invoke("clipboard_hide");
    else if (event.key === "ArrowDown") setActive(Math.min(index + 1, shown.length - 1));
    else if (event.key === "ArrowUp") setActive(Math.max(index - 1, 0));
    else if (event.key === "Enter") paste(current);
    else if (command && /^[1-9]$/.test(digit)) paste(shown[Number(digit) - 1]);
    else if (command && letter === "t") translate(current);
    else if (command && letter === "p") pin(current);
    else if (command && event.key === "Backspace") remove(current);
    else return;
    event.preventDefault();
  };
  // On the window, so the keys work wherever the focus is inside the board.
  const handler = useRef(onKeyDown);
  handler.current = onKeyDown;
  useEffect(() => {
    const pressed = (event: KeyboardEvent) => handler.current(event);
    const focused = () => search.current?.focus();
    window.addEventListener("keydown", pressed);
    window.addEventListener("focus", focused);
    return () => {
      window.removeEventListener("keydown", pressed);
      window.removeEventListener("focus", focused);
    };
  }, []);

  const row = (item: Item, position: number) => (
    <div
      key={item.id}
      className={position === index ? "board-item active" : "board-item"}
      onMouseMove={() => setActive(position)}
      onClick={() => paste(item)}
    >
      <span className="board-kind">{kind(item.text)}</span>
      <span className="board-line">{item.text.trim().slice(0, 200)}</span>
      {position < 9 && (
        <kbd>
          {MOD}
          {position + 1}
        </kbd>
      )}
    </div>
  );

  return (
    <div className="board">
      <div className="board-search">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
          <circle cx="11" cy="11" r="7" />
          <path d="M20 20l-3.5-3.5" />
        </svg>
        <input
          ref={search}
          autoFocus
          spellCheck={false}
          placeholder="Tìm trong clipboard"
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setActive(0);
          }}
        />
      </div>
      <div className="board-body">
        {shown.length === 0 ? (
          <div className="board-empty">{items.length === 0 ? "Chưa có gì. Copy một đoạn văn bản để bắt đầu." : "Không tìm thấy đoạn nào."}</div>
        ) : (
          <>
            <div className="board-list" ref={list}>
              {recent.length > 0 && <div className="board-section">Gần đây</div>}
              {recent.map((item, position) => row(item, position))}
              {pinned.length > 0 && <div className="board-section">Đã ghim</div>}
              {pinned.map((item, position) => row(item, recent.length + position))}
            </div>
            <div className="board-preview">
              <div className="board-full">{current?.text.slice(0, 4000)}</div>
              <div className="board-meta">
                {[current?.app, ago(current?.at ?? 0), `${current?.text.length ?? 0} ký tự`].filter(Boolean).join(" · ")}
              </div>
            </div>
          </>
        )}
      </div>
      <div className="board-foot">
        <button onClick={() => invoke("clipboard_clear")}>Xoá hết</button>
        <button onClick={() => paste(current)}>
          Dán<kbd>↵</kbd>
        </button>
        <button onClick={() => translate(current)}>
          Dịch<kbd>{MOD}T</kbd>
        </button>
        <button onClick={() => pin(current)}>
          {current?.pinned ? "Bỏ ghim" : "Ghim"}
          <kbd>{MOD}P</kbd>
        </button>
        <button onClick={() => remove(current)}>
          Xoá<kbd>{MOD}⌫</kbd>
        </button>
      </div>
    </div>
  );
}
