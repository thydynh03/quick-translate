import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ENGINE_NAMES, PopupState } from "./types";
import { AppLanguage, useT } from "./i18n";

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

import { IconBack, IconBookmark, IconBrand, IconCheck, IconClose, IconCopy, IconPin, IconSpeak } from "./icons";

/** The sentence with every occurrence of the word marked. */
function Highlight({ text, word }: { text: string; word: string }) {
  if (!word) return <>{text}</>;
  const escaped = word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  // Whole words only: "he" must not light up inside "the".
  const parts = text.split(new RegExp(`(?<![\\p{L}\\p{N}])(${escaped})(?![\\p{L}\\p{N}])`, "giu"));
  return (
    <>
      {parts.map((part, i) => (part.toLowerCase() === word.toLowerCase() ? <mark key={i}>{part}</mark> : part))}
    </>
  );
}

export default function Popup() {
  const [state, setState] = useState<PopupState>({ mode: "hidden" });
  const [pinned, setPinned] = useState(false);
  const [copied, setCopied] = useState(false);
  const [saved, setSaved] = useState(false);
  const [saveError, setSaveError] = useState("");
  const [trail, setTrail] = useState<{ text: string; context: string | null }[]>([]);
  const card = useRef<HTMLDivElement>(null);
  // The card the popup shows now; an older vocab_has answer must not change `saved`.
  const shownKey = useRef("");

  useEffect(() => {
    invoke<PopupState>("popup_state").then(setState);
    const unlisten = listen<PopupState>("popup-state", (event) => {
      setState(event.payload);
      setCopied(false);
      setSaved(false);
      setSaveError("");
      const next = event.payload;
      const key = next.mode === "result" ? `${next.result.to}|${next.source}` : "";
      shownKey.current = key;
      if (next.mode === "result") {
        invoke<boolean>("vocab_has", { front: next.source, to: next.result.to }).then((has) => {
          if (shownKey.current === key) setSaved(has);
        }, (error) => setSaveError(String(error)));
      }
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
  const [appLang, setAppLang] = useState<AppLanguage>("vi");
  useEffect(() => {
    const apply = () =>
      invoke<{ popup_font_size: number; popup_opacity: number; app_language?: string }>("get_settings").then((settings) => {
        const root = document.documentElement.style;
        root.fontSize = `${settings.popup_font_size}px`;
        root.setProperty("--popup-opacity", `${settings.popup_opacity}%`);
        setLook(`${settings.popup_font_size}/${settings.popup_opacity}`);
        if (settings.app_language === "en" || settings.app_language === "vi") {
          setAppLang(settings.app_language as AppLanguage);
        }
      });
    apply();
    const unlisten = listen("settings-changed", apply);
    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  const t = useT(appLang);

  // Tell the window how tall the rendered card is.
  useLayoutEffect(() => {
    if (!card.current) return;
    const height = Math.min(card.current.scrollHeight + 2, MAX_HEIGHT);
    invoke("popup_resize", { height });
  }, [state, look]);

  if (state.mode === "hidden") return null;

  if (state.mode === "icon") {
    return (
      <button className="trigger" onClick={() => invoke("popup_activate")} aria-label={t.popup_drag_hint}>
        <IconBrand size={18} />
      </button>
    );
  }

  const source = state.source;
  const result = state.mode === "result" ? state.result : null;
  const shownContext = result?.context?.sentence ?? null;
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
    setTrail((t) => [...t, { text: source, context: shownContext }]);
    // The sentence the word was clicked in gives it context.
    const sentence = result?.context?.sentence ?? source;
    invoke("popup_lookup", { text: clean, context: sentence.length <= 600 ? sentence : null });
  };

  const back = () => {
    const previous = trail[trail.length - 1];
    setTrail((t) => t.slice(0, -1));
    invoke("popup_lookup", { text: previous.text, context: previous.context });
  };

  const copy = async () => {
    if (!result) return;
    await invoke("copy_text", { text: result.text });
    setCopied(true);
  };

  // Saves the selection as a flashcard; a second press removes it.
  const toggleSave = async () => {
    if (!result) return;
    shownKey.current += "*"; // a pending vocab_has is now stale
    setSaveError("");
    try {
      if (saved) {
        await invoke("vocab_remove", { front: source, to: result.to });
        setSaved(false);
      } else {
        await saveCard();
        setSaved(true);
      }
    } catch (error) {
      setSaveError(String(error));
    }
  };

  const saveCard = async () => {
    if (!result) return;
    const senses = [...result.dict, ...result.definitions];
    const detail = [result.phonetic ?? "", ...senses.map((d) => `${d.pos}: ${d.terms.join(", ")}`)]
      .filter(Boolean)
      .join("\n");
    await invoke("vocab_add", { card: { at: 0, front: source, back: result.text, detail, from: result.from, to: result.to } });
  };

  const togglePin = () => {
    invoke("popup_pin", { pinned: !pinned });
    setPinned(!pinned);
  };

  return (
    <div className="card" ref={card}>
      <header className="bar">
        {trail.length > 0 && (
          <button className="tool" onClick={back} title={t.popup_back}>
            <IconBack size={16} />
          </button>
        )}
        <span className="route">
          {result ? (
            <>
              {result.from.toUpperCase()} <span className="arrow">→</span> {result.to.toUpperCase()}
              <span className="engine">{ENGINE_NAMES[result.engine] ?? result.engine}</span>
            </>
          ) : state.mode === "loading" ? (
            t.popup_loading
          ) : (
            t.popup_failed
          )}
        </span>
        <span className="tools">
          {result && (
            <>
              <button className="tool" onClick={() => speak(source, result.from)} title={t.popup_speak}>
                <IconSpeak size={16} />
              </button>
              <button className={saved ? "tool on" : "tool"} onClick={toggleSave} title={saved ? t.popup_saved : t.popup_save}>
                <IconBookmark size={16} filled={saved} />
              </button>
              <button className="tool" onClick={copy} title={t.popup_copy}>
                {copied ? <IconCheck size={16} /> : <IconCopy size={16} />}
              </button>
            </>
          )}
          <button className={pinned ? "tool on" : "tool"} onClick={togglePin} title={t.popup_pin}>
            <IconPin size={16} />
          </button>
          <button className="tool" onClick={() => invoke("popup_close")} title={t.popup_close}>
            <IconClose size={16} />
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

      {(state.mode === "loading" || (state.mode === "result" && state.partial)) && <div className="loading" />}
      {state.mode === "error" && <p className="error">{state.message}</p>}

      {result && (
        <>
          <p className="result">{result.text}</p>
          {result.context && (
            <div className="context">
              <p className="context-label">{result.context.meaning ? t.context_meaning : t.context_in_sentence}</p>
              {result.context.meaning && <p className="context-meaning">{result.context.meaning}</p>}
              <p className="context-sentence">
                <Highlight text={result.context.sentence} word={source.trim()} />
              </p>
              <p className="context-translation">{result.context.translation}</p>
            </div>
          )}
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
          {saveError && <p className="note">{appLang === "en" ? `Could not save card: ${saveError}` : `Không lưu được từ: ${saveError}`}</p>}
        </>
      )}
    </div>
  );
}
