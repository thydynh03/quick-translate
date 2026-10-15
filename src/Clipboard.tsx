import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { AppLanguage, useT } from "./i18n";

type Item = {
  id: number;
  text: string;
  pinned: boolean;
  /** True when this entry is a screenshot / image copied to the clipboard. */
  image?: boolean;
  /** Base64 data URL (data:image/png;base64,...) for screenshot preview. */
  image_data?: string | null;
  /** Rich text HTML when available, preserving fonts, colors, and styles. */
  html?: string | null;
  app: string | null;
  at: number;
};

const MAC = navigator.userAgent.includes("Mac");
/** How the command key is written on this system. */
const MOD = MAC ? "⌘" : "Ctrl+";

/** Detect what kind of content the text is. */
type ContentKind = "image" | "rich" | "url" | "code" | "text";

function detectKind(item: Item): ContentKind {
  if (item.image) return "image";
  if (item.html && item.html.trim().length > 0 && /<[a-z][\s\S]*>/i.test(item.html)) return "rich";
  const trimmed = item.text.trim();
  if (/^(https?:\/\/|www\.)\S+$/i.test(trimmed)) return "url";
  if (looksLikeCode(trimmed)) return "code";
  return "text";
}

function kindBadge(kind: ContentKind): string {
  if (kind === "image") return "📷";
  if (kind === "rich") return "Aa";
  if (kind === "url") return "↗";
  if (kind === "code") return "</>";
  return "Aa";
}

/** Heuristics to decide if the text looks like code. */
function looksLikeCode(text: string): boolean {
  const lines = text.split("\n");
  // Multiple lines with consistent indentation
  const indented = lines.filter((l) => /^[ \t]{2,}/.test(l)).length;
  if (indented >= 2) return true;
  // Common code patterns
  if (
    /[{};]\s*$|=>\s*\{|^\s*(const|let|var|import|export|from|def|fn|function|class|#include|#define|package|using|namespace|public|private|protected|void|int|str|bool)\s+/m.test(
      text
    )
  )
    return true;
  // Shell commands
  if (/^\s*\$\s+\w/.test(text)) return true;
  // HTML/XML
  if (/<\/?[a-z][\w-]*(\s[^>]*)?>/.test(text)) return true;
  return false;
}

// ---------------------------------------------------------------- syntax highlight

type Token = { type: "keyword" | "string" | "comment" | "number" | "operator" | "plain" | "tag" | "attr"; value: string };

const KEYWORDS = new Set([
  "const","let","var","function","return","if","else","for","while","do","switch","case","break",
  "continue","import","export","from","default","class","new","this","super","typeof","instanceof",
  "void","null","undefined","true","false","async","await","try","catch","finally","throw",
  "def","fn","pub","mut","let","use","mod","impl","struct","enum","trait","where","match",
  "int","str","bool","float","double","char","string","None","Some","Ok","Err",
  "package","import","public","private","protected","static","final","extends","implements",
  "#include","#define","#ifdef","#endif","#pragma",
  "and","or","not","in","is","lambda","pass","del","with","yield","global","nonlocal","assert",
  "SELECT","FROM","WHERE","INSERT","UPDATE","DELETE","JOIN","ON","AS","CREATE","TABLE","INDEX",
]);

function tokenize(code: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;
  while (i < code.length) {
    if ((code[i] === "/" && code[i + 1] === "/") || code[i] === "#") {
      const end = code.indexOf("\n", i);
      const value = end === -1 ? code.slice(i) : code.slice(i, end);
      tokens.push({ type: "comment", value });
      i += value.length;
      continue;
    }
    if (code[i] === "/" && code[i + 1] === "*") {
      const end = code.indexOf("*/", i + 2);
      const value = end === -1 ? code.slice(i) : code.slice(i, end + 2);
      tokens.push({ type: "comment", value });
      i += value.length;
      continue;
    }
    if (code[i] === '"' || code[i] === "'" || code[i] === "`") {
      const q = code[i];
      let j = i + 1;
      while (j < code.length && code[j] !== q) {
        if (code[j] === "\\") j++;
        j++;
      }
      tokens.push({ type: "string", value: code.slice(i, j + 1) });
      i = j + 1;
      continue;
    }
    if (/\d/.test(code[i]) || (code[i] === "-" && /\d/.test(code[i + 1] ?? ""))) {
      let j = i + 1;
      while (j < code.length && /[\d._xXa-fA-F]/.test(code[j])) j++;
      tokens.push({ type: "number", value: code.slice(i, j) });
      i = j;
      continue;
    }
    if (/[a-zA-Z_$]/.test(code[i])) {
      let j = i + 1;
      while (j < code.length && /[\w$]/.test(code[j])) j++;
      const word = code.slice(i, j);
      tokens.push({ type: KEYWORDS.has(word) ? "keyword" : "plain", value: word });
      i = j;
      continue;
    }
    if (code[i] === "<" && /[a-zA-Z/!]/.test(code[i + 1] ?? "")) {
      const end = code.indexOf(">", i);
      const value = end === -1 ? code.slice(i) : code.slice(i, end + 1);
      tokens.push({ type: "tag", value });
      i += value.length;
      continue;
    }
    if (/[+\-*/%=&|^~<>!?:;,()[\]{}.]/.test(code[i])) {
      tokens.push({ type: "operator", value: code[i] });
      i++;
      continue;
    }
    tokens.push({ type: "plain", value: code[i] });
    i++;
  }
  return tokens;
}

function CodePreview({ code }: { code: string }) {
  const lines = code.split("\n");
  return (
    <div className="preview-code">
      {lines.map((line, li) => {
        const tokens = tokenize(line);
        return (
          <div key={li} className="preview-code-line">
            <span className="preview-line-num">{li + 1}</span>
            <span className="preview-line-body">
              {tokens.map((tok, ti) => (
                <span key={ti} className={`tok-${tok.type}`}>
                  {tok.value}
                </span>
              ))}
            </span>
          </div>
        );
      })}
    </div>
  );
}

function UrlPreview({ url }: { url: string }) {
  return (
    <div className="preview-url">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
        <path d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71" />
        <path d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71" />
      </svg>
      <span className="preview-url-text">{url}</span>
    </div>
  );
}

function ImagePreview({ src, text, app }: { src?: string | null; text?: string; app: string | null }) {
  if (src) {
    return (
      <div className="preview-image-wrap">
        <div className="preview-image-canvas">
          <img src={src} alt="Clipboard capture" className="preview-image-img" />
        </div>
        <div className="preview-image-footer">
          <span className="preview-image-tag">{text || "Ảnh chụp màn hình"}</span>
          {app && <span className="preview-image-source">{app}</span>}
        </div>
      </div>
    );
  }
  return (
    <div className="preview-image">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <rect x="3" y="3" width="18" height="18" rx="3" ry="3" />
        <circle cx="8.5" cy="8.5" r="1.5" />
        <polyline points="21,15 16,10 5,21" />
      </svg>
      <div className="preview-image-label">
        <strong>Ảnh trong clipboard</strong>
        {app && <span>{app}</span>}
      </div>
    </div>
  );
}

/** Renders rich text HTML while preserving fonts, styles, and colors. */
function HtmlPreview({ html }: { html: string }) {
  const clean = useMemo(() => {
    return html
      .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, "")
      .replace(/<iframe\b[^<]*(?:(?!<\/iframe>)<[^<]*)*<\/iframe>/gi, "");
  }, [html]);

  return (
    <div className="preview-html-wrap">
      <div className="preview-html-body" dangerouslySetInnerHTML={{ __html: clean }} />
    </div>
  );
}

function RichText({ text }: { text: string }) {
  const parts: Array<{ type: "bold" | "italic" | "code" | "plain"; value: string }> = [];
  const pattern = /\*\*(.+?)\*\*|__(.+?)__|_(.+?)_|\*(.+?)\*|`(.+?)`/g;
  let last = 0;
  let match;
  while ((match = pattern.exec(text)) !== null) {
    if (match.index > last) parts.push({ type: "plain", value: text.slice(last, match.index) });
    if (match[1] || match[2]) parts.push({ type: "bold", value: match[1] ?? match[2] });
    else if (match[3] || match[4]) parts.push({ type: "italic", value: match[3] ?? match[4] });
    else if (match[5]) parts.push({ type: "code", value: match[5] });
    last = match.index + match[0].length;
  }
  if (last < text.length) parts.push({ type: "plain", value: text.slice(last) });

  return (
    <>
      {parts.map((p, i) => {
        if (p.type === "bold") return <strong key={i}>{p.value}</strong>;
        if (p.type === "italic") return <em key={i}>{p.value}</em>;
        if (p.type === "code") return <code key={i} className="preview-inline-code">{p.value}</code>;
        return <span key={i}>{p.value}</span>;
      })}
    </>
  );
}

function TextPreview({ text }: { text: string }) {
  const lines = text.split("\n");
  return (
    <div className="preview-text">
      {lines.map((line, i) => (
        <p key={i} className="preview-text-line">
          {line ? <RichText text={line} /> : <br />}
        </p>
      ))}
    </div>
  );
}

/** The right-hand preview pane that adapts its look to the item's content. */
function RichPreview({ item, appLang, t }: { item: Item | undefined; appLang: AppLanguage; t: ReturnType<typeof useT> }) {
  if (!item) return <div className="board-preview" />;
  const kind = detectKind(item);
  const content = (() => {
    if (kind === "image") return <ImagePreview src={item.image_data} text={item.text} app={item.app} />;
    if (kind === "rich" && item.html) return <HtmlPreview html={item.html} />;
    if (kind === "url") return <UrlPreview url={item.text.trim()} />;
    if (kind === "code") return <CodePreview code={item.text.slice(0, 8000)} />;
    return <TextPreview text={item.text.slice(0, 4000)} />;
  })();
  return (
    <div className={`board-preview preview-kind-${kind}`}>
      <div className="board-full">{content}</div>
      <div className="board-meta">
        {[item.app, ago(item.at, appLang), !item.image ? t.clipboard_chars(item.text.length) : undefined].filter(Boolean).join(" · ")}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------- timing

function ago(at: number, lang: AppLanguage): string {
  if (!at) return "";
  const minutes = Math.floor((Date.now() - at) / 60000);
  const isEn = lang === "en";
  if (minutes < 1) return isEn ? "just now" : "vừa xong";
  if (minutes < 60) return isEn ? `${minutes}m ago` : `${minutes} phút trước`;
  if (minutes < 60 * 24) return isEn ? `${Math.floor(minutes / 60)}h ago` : `${Math.floor(minutes / 60)} giờ trước`;
  return isEn ? `${Math.floor(minutes / 60 / 24)}d ago` : `${Math.floor(minutes / 60 / 24)} ngày trước`;
}

// ---------------------------------------------------------------- main component

export default function Clipboard() {
  const [items, setItems] = useState<Item[]>([]);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [appLang, setAppLang] = useState<AppLanguage>("vi");
  const search = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const follow = useRef<number | null>(null);

  // Splitter state: ratio in percent for the left list column
  const [splitRatio, setSplitRatio] = useState<number>(() => {
    const saved = localStorage.getItem("swipelate_clipboard_split");
    return saved ? Math.min(Math.max(Number(saved), 20), 75) : 45;
  });
  const isResizingSplit = useRef(false);

  const startSplitResize = (e: React.MouseEvent) => {
    e.preventDefault();
    isResizingSplit.current = true;
    const onMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizingSplit.current) return;
      const bodyEl = document.querySelector(".board-body");
      if (!bodyEl) return;
      const rect = bodyEl.getBoundingClientRect();
      if (rect.width === 0) return;
      const ratio = ((moveEvent.clientX - rect.left) / rect.width) * 100;
      const clamped = Math.min(Math.max(ratio, 20), 75);
      setSplitRatio(clamped);
      localStorage.setItem("swipelate_clipboard_split", clamped.toFixed(1));
    };
    const onMouseUp = () => {
      isResizingSplit.current = false;
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  };

  // 4 corners and 4 edges resize handlers
  const handleEdgeResize = (direction: "North" | "South" | "East" | "West" | "NorthEast" | "NorthWest" | "SouthEast" | "SouthWest") => (e: React.MouseEvent) => {
    e.preventDefault();
    try {
      getCurrentWindow().startResizeDragging(direction);
    } catch {
      // Fallback if not supported
    }
  };

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
  const translate = (item?: Item) => item && !item.image && invoke("clipboard_translate", { id: item.id });
  const pin = (item?: Item) => {
    if (!item) return;
    follow.current = item.id;
    invoke("clipboard_pin", { id: item.id, pinned: !item.pinned });
  };
  const remove = (item?: Item) => item && invoke("clipboard_remove", { id: item.id });

  const onKeyDown = (event: KeyboardEvent) => {
    const command = MAC ? event.metaKey : event.ctrlKey;
    const moving = event.key === "ArrowDown" || event.key === "ArrowUp";
    if (event.isComposing || (event.repeat && !moving)) return;
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

  const row = (item: Item, position: number) => {
    const k = detectKind(item);
    return (
      <div
        key={item.id}
        className={position === index ? "board-item active" : "board-item"}
        onMouseMove={() => select(position)}
        onClick={() => paste(item)}
      >
        <span className={`board-kind kind-${k}`}>
          {item.image && item.image_data ? (
            <img src={item.image_data} alt="" className="board-kind-thumb" />
          ) : (
            kindBadge(k)
          )}
        </span>
        <span className="board-line">{item.text.trim().slice(0, 200)}</span>
        {position < 9 && (
          <kbd>
            {MOD}
            {position + 1}
          </kbd>
        )}
      </div>
    );
  };

  return (
    <div className="board">
      {/* 4 Edges for window resizing */}
      <div className="board-resizer edge-n" onMouseDown={handleEdgeResize("North")} />
      <div className="board-resizer edge-s" onMouseDown={handleEdgeResize("South")} />
      <div className="board-resizer edge-w" onMouseDown={handleEdgeResize("West")} />
      <div className="board-resizer edge-e" onMouseDown={handleEdgeResize("East")} />
      {/* 4 Corners for window resizing */}
      <div className="board-resizer corner-nw" onMouseDown={handleEdgeResize("NorthWest")} />
      <div className="board-resizer corner-ne" onMouseDown={handleEdgeResize("NorthEast")} />
      <div className="board-resizer corner-sw" onMouseDown={handleEdgeResize("SouthWest")} />
      <div className="board-resizer corner-se" onMouseDown={handleEdgeResize("SouthEast")} />

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
      <div
        className="board-body"
        style={{ gridTemplateColumns: `${splitRatio}% 6px minmax(0, 1fr)` }}
      >
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
            {/* Splitter bar in the middle: drag to resize columns */}
            <div
              className="board-splitter"
              onMouseDown={startSplitResize}
              title="Kéo sang trái/phải để thay đổi kích thước"
            >
              <div className="board-splitter-handle" />
            </div>
            <RichPreview item={current} appLang={appLang} t={t} />
          </>
        )}
      </div>
      <div className="board-foot">
        <button onClick={() => invoke("clipboard_clear")}>{t.clipboard_clear}</button>
        <button onClick={() => paste(current)}>
          {t.clipboard_paste}<kbd>↵</kbd>
        </button>
        {!current?.image && (
          <button onClick={() => translate(current)}>
            {t.clipboard_translate}<kbd>{MOD}T</kbd>
          </button>
        )}
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
