import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ENGINE_NAMES, PopupState } from "./types";

const MAX_HEIGHT = 460;

function speak(text: string, lang: string) {
  const utterance = new SpeechSynthesisUtterance(text);
  utterance.lang = lang;
  speechSynthesis.cancel();
  speechSynthesis.speak(utterance);
}

/** Splits text into words and the gaps between them, keeping both. */
function tokens(text: string): string[] {
  return text.split(/(\s+)/).filter((t) => t.length > 0);
}

function Icon({ d }: { d: string }) {
  return (
    <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={d} />
    </svg>
  );
}

const ICONS = {
  speak: "M11 5 6 9H3v6h3l5 4V5zM15.5 8.5a5 5 0 0 1 0 7M18.5 5.5a9 9 0 0 1 0 13",
  copy: "M9 9h10v11H9zM5 15V4h10",
  check: "M5 12l5 5L20 7",
  pin: "M9 3h6l-1 6 4 4H6l4-4-1-6zM12 13v8",
  close: "M6 6l12 12M18 6 6 18",
  back: "M15 5l-7 7 7 7",
};

export default function Popup() {
  const [state, setState] = useState<PopupState>({ mode: "hidden" });
  const [pinned, setPinned] = useState(false);
  const [copied, setCopied] = useState(false);
  const [trail, setTrail] = useState<string[]>([]);
  const card = useRef<HTMLDivElement>(null);

  useEffect(() => {
    invoke<PopupState>("popup_state").then(setState);
    const unlisten = listen<PopupState>("popup-state", (event) => {
      setState(event.payload);
      setCopied(false);
      if (event.payload.mode === "hidden" || event.payload.mode === "icon") {
        setPinned(false);
        setTrail([]);
      }
    });
    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  // Text size and opacity come from the settings.
  const [look, setLook] = useState("");
  useEffect(() => {
    const apply = () =>
      invoke<{ popup_font_size: number; popup_opacity: number }>("get_settings").then((settings) => {
        const root = document.documentElement.style;
        root.fontSize = `${settings.popup_font_size}px`;
        root.setProperty("--popup-opacity", `${settings.popup_opacity}%`);
        setLook(`${settings.popup_font_size}/${settings.popup_opacity}`);
      });
    apply();
    const unlisten = listen("settings-changed", apply);
    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  // Tell the window how tall the rendered card is.
  useLayoutEffect(() => {
    if (!card.current) return;
    const height = Math.min(card.current.scrollHeight + 2, MAX_HEIGHT);
    invoke("popup_resize", { height });
  }, [state, look]);

  if (state.mode === "hidden") return null;

  if (state.mode === "icon") {
    return (
      <button className="trigger" onClick={() => invoke("popup_activate")} aria-label="Dịch vùng bôi đen">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M4 5h8M8 3v2M5.5 5c.5 3 2.5 5.500 5.500 7M10.500 5C10 8 8 10.500 4.500 12" />
          <path d="M13 20l4-9 4 9M14.500 17h5" />
        </svg>
      </button>
    );
  }

  const source = state.source;
  const result = state.mode === "result" ? state.result : null;
  const words = tokens(source);
  const clickable = words.filter((w) => w.trim()).length > 1;

  // Cambridge has an English–Vietnamese dictionary; other pairs get the English one.
  const cambridge =
    result && !clickable && result.from === "en"
      ? `https://dictionary.cambridge.org/dictionary/${result.to === "vi" ? "english-vietnamese" : "english"}/${encodeURIComponent(source.trim().toLowerCase())}`
      : null;

  const oxford = `https://www.oxfordlearnersdictionaries.com/definition/english/${encodeURIComponent(source.trim().toLowerCase())}`;

  const lookup = (word: string) => {
    const clean = word.replace(/^[^\p{L}\p{N}]+|[^\p{L}\p{N}]+$/gu, "");
    if (!clean) return;
    setTrail((t) => [...t, source]);
    invoke("popup_lookup", { text: clean });
  };

  const back = () => {
    const previous = trail[trail.length - 1];
    setTrail((t) => t.slice(0, -1));
    invoke("popup_lookup", { text: previous });
  };

  const copy = async () => {
    if (!result) return;
    await invoke("copy_text", { text: result.text });
    setCopied(true);
  };

  const togglePin = () => {
    invoke("popup_pin", { pinned: !pinned });
    setPinned(!pinned);
  };

  return (
    <div className="card" ref={card}>
      <header className="bar">
        {trail.length > 0 && (
          <button className="tool" onClick={back} title="Quay lại">
            <Icon d={ICONS.back} />
          </button>
        )}
        <span className="route">
          {result ? (
            <>
              {result.from.toUpperCase()} <span className="arrow">→</span> {result.to.toUpperCase()}
              <span className="engine">{ENGINE_NAMES[result.engine] ?? result.engine}</span>
            </>
          ) : state.mode === "loading" ? (
            "Đang dịch…"
          ) : (
            "Không dịch được"
          )}
        </span>
        <span className="tools">
          {result && (
            <>
              <button className="tool" onClick={() => speak(source, result.from)} title="Đọc văn bản gốc">
                <Icon d={ICONS.speak} />
              </button>
              <button className="tool" onClick={copy} title="Sao chép bản dịch">
                <Icon d={copied ? ICONS.check : ICONS.copy} />
              </button>
            </>
          )}
          <button className={pinned ? "tool on" : "tool"} onClick={togglePin} title="Ghim">
            <Icon d={ICONS.pin} />
          </button>
          <button className="tool" onClick={() => invoke("popup_close")} title="Đóng">
            <Icon d={ICONS.close} />
          </button>
        </span>
      </header>

      {source && (
        <p className="source">
          {clickable
            ? words.map((word, i) =>
                word.trim() ? (
                  <span key={i} className="word" onClick={() => lookup(word)}>
                    {word}
                  </span>
                ) : (
                  word
                ),
              )
            : source}
          {result?.phonetic && <span className="phonetic"> /{result.phonetic}/</span>}
        </p>
      )}

      {state.mode === "loading" && <div className="loading" />}
      {state.mode === "error" && <p className="error">{state.message}</p>}

      {result && (
        <>
          <p className="result">{result.text}</p>
          {result.dict.length > 0 && (
            <dl className="dict">
              {result.dict.map((entry) => (
                <div key={entry.pos}>
                  <dt>{entry.pos}</dt>
                  <dd>{entry.terms.join(", ")}</dd>
                </div>
              ))}
            </dl>
          )}
          {result.definitions.length > 0 && (
            <dl className="dict">
              {result.definitions.map((entry) => (
                <div key={entry.pos}>
                  <dt>{entry.pos}</dt>
                  <dd>{entry.terms.join(" · ")}</dd>
                </div>
              ))}
            </dl>
          )}
          {cambridge && (
            <button className="link" onClick={() => openUrl(cambridge)}>
              Cambridge
            </button>
          )}
          {cambridge && (
            <button className="link" onClick={() => openUrl(oxford)}>
              Oxford
            </button>
          )}
          {result.note && <p className="note">{result.note}</p>}
        </>
      )}
    </div>
  );
}
