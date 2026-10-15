import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AppLanguage, useT } from "./i18n";

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

function ago(at: number, lang: AppLanguage): string {
  if (!at) return "";
  const minutes = Math.floor((Date.now() - at) / 60000);
  const isEn = lang === "en";
  if (minutes < 1) return isEn ? "just now" : "vừa xong";
  if (minutes < 60) return isEn ? `${minutes}m ago` : `${minutes} phút trước`;
  if (minutes < 60 * 24) return isEn ? `${Math.floor(minutes / 60)}h ago` : `${Math.floor(minutes / 60)} giờ trước`;
  return isEn ? `${Math.floor(minutes / 60 / 24)}d ago` : `${Math.floor(minutes / 60 / 24)} ngày trước`;
}

/** The texts copied lately, laid out like Spotlight: pick one to paste it where you were typing. */
export default function Clipboard() {
  const [items, setItems] = useState<Item[]>([]);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [appLang, setAppLang] = useState<AppLanguage>("vi");
  const search = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const follow = useRef<number | null>(null);

  const load = () => invoke<Item[]>("clipboard_list").then(setItems);

  useEffect(() => {
    load();
    invoke<{ app_language?: string }>("get_settings").then((s) => {
      if (s.app_language === "en" || s.app_language === "vi") setAppLang(s.app_language as AppLanguage);
    }).catch(() => {});
    const opened = listen("clipboard-open", () => {
      setQuery("");
      setActive(0);
      load();
      invoke<{ app_language?: string }>("get_settings").then((s) => {
        if (s.app_language === "en" || s.app_language === "vi") setAppLang(s.app_language as AppLanguage);
      }).catch(() => {});
      search.current?.focus();
    });
    const changed = listen("clipboard-changed", load);
    return () => {
      opened.then((off) => off());
      changed.then((off) => off());
    };
  }, []);

  const t = useT(appLang);

  // The newest first, then the pinned ones.
  const [recent, pinned] = useMemo(() => {
    const wanted = query.trim().toLowerCase();
    const found = wanted ? items.filter((item) => item.text.toLowerCase().includes(wanted)) : items;
    return [found.filter((item) => !item.pinned), found.filter((item) => item.pinned)];
  }, [items, query]);
  const shown = useMemo(() => [...recent, ...pinned], [recent, pinned]);
  useEffect(() => {
    const moved = shown.findIndex((item) => item.id === follow.current);
    if (moved >= 0) setActive(moved);
  }, [shown]);
  const index = Math.min(active, Math.max(shown.length - 1, 0));
  const current: Item | undefined = shown[index];

  useEffect(() => {
    list.current?.querySelector(".board-item.active")?.scrollIntoView({ block: "nearest" });
  }, [index, shown]);

  const select = (position: number) => {
    follow.current = null;
    setActive(position);
  };
  const paste = (item?: Item) => item && invoke("clipboard_paste", { id: item.id });
  const translate = (item?: Item) => item && invoke("clipboard_translate", { id: item.id });
  const pin = (item?: Item) => {
    if (!item) return;
    // It changes group: the selection follows it.
    follow.current = item.id;
    invoke("clipboard_pin", { id: item.id, pinned: !item.pinned });
  };
  const remove = (item?: Item) => item && invoke("clipboard_remove", { id: item.id });

  const onKeyDown = (event: KeyboardEvent) => {
    const command = MAC ? event.metaKey : event.ctrlKey;
    const moving = event.key === "ArrowDown" || event.key === "ArrowUp";
    // A held key acts once, and the Enter that ends a composed word is not ours.
    if (event.isComposing || (event.repeat && !moving)) return;
    // By position on the keyboard when known, so other layouts work; by character otherwise.
    const digit = /^Digit\d$/.test(event.code) ? event.code.slice(5) : event.key;
    const letter = /^Key[A-Z]$/.test(event.code) ? event.code.slice(3).toLowerCase() : event.key.toLowerCase();
    if (event.key === "Escape") invoke("clipboard_hide");
    else if (event.key === "ArrowDown") select(Math.min(index + 1, shown.length - 1));
    else if (event.key === "ArrowUp") select(Math.max(index - 1, 0));
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
      onMouseMove={() => select(position)}
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
          placeholder={t.clipboard_search_ph}
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setActive(0);
          }}
        />
      </div>
      <div className="board-body">
        {shown.length === 0 ? (
          <div className="board-empty">{items.length === 0 ? t.clipboard_empty : t.clipboard_no_matches}</div>
        ) : (
          <>
            <div className="board-list" ref={list}>
              {recent.length > 0 && <div className="board-section">{t.clipboard_recent}</div>}
              {recent.map((item, position) => row(item, position))}
              {pinned.length > 0 && <div className="board-section">{t.clipboard_pinned}</div>}
              {pinned.map((item, position) => row(item, recent.length + position))}
            </div>
            <div className="board-preview">
              <div className="board-full">{current?.text.slice(0, 4000)}</div>
              <div className="board-meta">
                {[current?.app, ago(current?.at ?? 0, appLang), t.clipboard_chars(current?.text.length ?? 0)].filter(Boolean).join(" · ")}
              </div>
            </div>
          </>
        )}
      </div>
      <div className="board-foot">
        <button onClick={() => invoke("clipboard_clear")}>{t.clipboard_clear}</button>
        <button onClick={() => paste(current)}>
          {t.clipboard_paste}<kbd>↵</kbd>
        </button>
        <button onClick={() => translate(current)}>
          {t.clipboard_translate}<kbd>{MOD}T</kbd>
        </button>
        <button onClick={() => pin(current)}>
          {current?.pinned ? t.clipboard_unpin : t.clipboard_pin}
          <kbd>{MOD}P</kbd>
        </button>
        <button onClick={() => remove(current)}>
          {t.clipboard_remove}<kbd>{MOD}⌫</kbd>
        </button>
      </div>
    </div>
  );
}
