import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ENGINE_NAMES, HistoryEntry, UpdateInfo, LANGUAGES, Profile, Settings as Config, Translation, langName } from "./types";
import OfflineModels from "./OfflineModels";
import Subtitles from "./Subtitles";
import Vocab from "./Vocab";
import { IconAbout, IconEngine, IconGeneral, IconGlobe, IconHistory, IconMoon, IconSubtitles, IconSun, IconVocab } from "./icons";
import { AppLanguage, useT } from "./i18n";

type Tab = "general" | "engine" | "subtitles" | "vocab" | "history" | "about";

const TAB_ICONS: Record<Tab, React.ReactNode> = {
  general: <IconGeneral size={15} />,
  engine: <IconEngine size={15} />,
  subtitles: <IconSubtitles size={15} />,
  vocab: <IconVocab size={15} />,
  history: <IconHistory size={15} />,
  about: <IconAbout size={15} />,
};

const SITE = "https://swipelate.com/";
const REPO = "https://github.com/thydynh03/swipelate";

function About({
  checkUpdates,
  setCheckUpdates,
  lang = "vi",
}: {
  checkUpdates: boolean;
  setCheckUpdates: (on: boolean) => void;
  lang?: AppLanguage;
}) {
  const t = useT(lang);
  const [version, setVersion] = useState("");
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  // Installed from the Microsoft Store, which does the updating.
  const [store, setStore] = useState(false);

  const check = async () => {
    setBusy(true);
    setStatus(t.checking);
    try {
      const info = await invoke<UpdateInfo>("update_check");
      setUpdate(info);
      setStatus(info.available ? "" : t.up_to_date(info.current));
    } catch (error) {
      setStatus(lang === "en" ? `Check failed: ${error}` : `Không kiểm tra được: ${error}`);
    } finally {
      setBusy(false);
    }
  };

  const install = async () => {
    setBusy(true);
    setProgress(0);
    setStatus(lang === "en" ? "Downloading installer…" : "Đang tải bộ cài…");
    try {
      await invoke("update_install");
      // Windows quits here to run the installer; macOS opens the disk image.
      setStatus(
        lang === "en"
          ? "Installer opened. Drag Swipelate to Applications to replace older version, then restart app."
          : "Đã mở bộ cài. Kéo Swipelate vào Applications để thay bản cũ, rồi mở lại app. Còn Quick Translate (tên cũ) trong Applications thì xoá đi."
      );
    } catch (error) {
      setStatus(lang === "en" ? `Update failed: ${error}` : `Không cập nhật được: ${error}`);
    } finally {
      setBusy(false);
      setProgress(null);
    }
  };

  useEffect(() => {
    getVersion().then(setVersion);
    invoke<boolean>("store_build").then((packaged) => {
      setStore(packaged);
      if (checkUpdates && !packaged) check();
    });
    const progressed = listen<{ done: number; total: number }>("update-progress", (event) => {
      const { done, total } = event.payload;
      setProgress(total ? Math.round((done / total) * 100) : null);
    });
    return () => {
      progressed.then((off) => off());
    };
  }, []);

  return (
    <section>
      <h2>{t.about_title}</h2>
      <p>{t.about_version(version)}</p>

      {store ? (
        <p className="help">{t.store_notice}</p>
      ) : (
        <div className="update">
          {update?.available ? (
            <>
              <p>
                <strong>{t.new_version_available(update.latest)}</strong>
                {update.published && (
                  <small> · {new Date(update.published).toLocaleDateString(lang === "en" ? "en-US" : "vi-VN")}</small>
                )}
              </p>
              {update.notes && <pre className="notes">{update.notes}</pre>}
              <div className="actions">
                <button className="primary" onClick={install} disabled={busy}>
                  {t.downloading_installer(progress)}
                </button>
                <button onClick={() => openUrl(update.page)}>{t.view_github}</button>
              </div>
              <p className="help">{t.about_notice}</p>
            </>
          ) : (
            <div className="actions">
              <button onClick={check} disabled={busy}>
                {t.check_btn}
              </button>
            </div>
          )}
          {status && <p className="help">{status}</p>}
          <label className="row">
            <span>{t.auto_check_update}</span>
            <input type="checkbox" checked={checkUpdates} onChange={(e) => setCheckUpdates(e.target.checked)} />
          </label>
        </div>
      )}

      <p className="help">{t.about_license_text}</p>
      <p className="help">{t.about_dict_license}</p>
      <div className="actions">
        <button onClick={() => openUrl(`${SITE}docs.html`)}>{t.about_docs}</button>
        <button onClick={() => openUrl(`${SITE}license.html`)}>{t.about_license}</button>
        <button onClick={() => openUrl(`${SITE}privacy.html`)}>{t.about_privacy}</button>
        <button onClick={() => openUrl(`${REPO}/issues`)}>{t.about_report}</button>
      </div>
    </section>
  );
}

function getEngines(lang: AppLanguage = "vi"): { id: string; hint: string }[] {
  const isEn = lang === "en";
  return [
    {
      id: "google",
      hint: isEn
        ? "No key needed. Uses unofficial web endpoint, may be rate limited."
        : "Không cần key. Dùng endpoint web không chính thức, có thể bị giới hạn.",
    },
    { id: "deepl", hint: isEn ? "Requires DeepL API key." : "Cần API key của DeepL." },
    {
      id: "claude",
      hint: isEn
        ? "Requires Anthropic API key. More natural for long sentences, slightly slower."
        : "Cần API key của Anthropic. Dịch tự nhiên hơn với câu dài, chậm hơn.",
    },
    { id: "offline", hint: isEn ? "Runs locally, no internet needed. Download models first." : "Chạy trên máy, không cần mạng. Cần tải model trước." },
    {
      id: "openai",
      hint: isEn
        ? "ChatGPT, Gemini or local models (Ollama, LM Studio). Fill in API address & model below."
        : "ChatGPT, Gemini hoặc model chạy trên máy (Ollama, LM Studio). Điền địa chỉ API và model bên dưới.",
    },
  ];
}

/** Builds a shortcut string such as "Ctrl+Shift+D" from a key event. */
function shortcutFrom(event: React.KeyboardEvent): string | null {
  const key = event.key;
  if (["Control", "Shift", "Alt", "Meta"].includes(key)) return null;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Super");
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

export default function Settings() {
  const [tab, setTab] = useState<Tab>("general");
  // The version the app was updated from, until the notice about it is dismissed.
  const [updatedFrom, setUpdatedFrom] = useState<string | null>(null);
  const [version, setVersion] = useState("");
  useEffect(() => {
    getVersion().then(setVersion);
    invoke<string | null>("updated_from").then((previous) => {
      if (previous === null) return;
      setUpdatedFrom(previous);
      setTab("about");
    });
  }, []);
  const [config, setConfig] = useState<Config | null>(null);
  const [autostart, setAutostart] = useState(false);
  const [status, setStatus] = useState("");
  const [sample, setSample] = useState("The quick brown fox jumps over the lazy dog.");
  const [sampleResult, setSampleResult] = useState<Translation | null>(null);
  const [sampleError, setSampleError] = useState("");
  const [busy, setBusy] = useState(false);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [historyQuery, setHistoryQuery] = useState("");
  const [historyVisibleCount, setHistoryVisibleCount] = useState(40);

  const load = () => invoke<Config>("get_settings").then(setConfig);

  useEffect(() => {
    const opened = listen("show-update", () => setTab("about"));
    return () => {
      opened.then((off) => off());
    };
  }, []);

  useEffect(() => {
    load();
    invoke<boolean>("autostart_get").then(setAutostart).catch(() => {});
    const unlisten = listen("settings-changed", load);
    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  useEffect(() => {
    if (tab === "history") {
      invoke<HistoryEntry[]>("history_list").then(setHistory);
      setHistoryVisibleCount(40);
    }
  }, [tab]);

  if (!config) return null;

  const appLang: AppLanguage = (config.app_language as AppLanguage) || "vi";
  const t = useT(appLang);

  const historyNeedle = historyQuery.trim().toLowerCase();
  const filteredHistory = history.filter((entry) => `${entry.source}\n${entry.text}`.toLowerCase().includes(historyNeedle));

  const update = async (patch: Partial<Config>) => {
    const next = { ...config, ...patch };
    setConfig(next);
    try {
      const failed = await invoke<string[]>("save_settings", { settings: next });
      setStatus(failed.length ? t.status_hotkey_conflict(failed.join(", ")) : t.status_saved);
    } catch (error) {
      setStatus(String(error));
    }
  };

  const toggleAutostart = async (on: boolean) => {
    try {
      setAutostart(await invoke<boolean>("autostart_set", { on }));
    } catch (error) {
      setStatus(String(error));
    }
  };

  const runSample = async () => {
    setBusy(true);
    setSampleError("");
    setSampleResult(null);
    try {
      setSampleResult(await invoke<Translation>("translate_now", { text: sample }));
    } catch (error) {
      setSampleError(String(error));
    } finally {
      setBusy(false);
    }
  };

  const isDark =
    config.theme === "dark" ||
    (config.theme === "system" &&
      typeof window !== "undefined" &&
      window.matchMedia &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);

  const toggleTheme = (e: React.MouseEvent) => {
    const next = isDark ? "light" : "dark";
    const ripple = document.createElement("div");
    ripple.className = "theme-ripple";
    ripple.style.left = `${e.clientX}px`;
    ripple.style.top = `${e.clientY}px`;
    ripple.style.background = next === "dark" ? "radial-gradient(circle, #1d1f24 35%, transparent 70%)" : "radial-gradient(circle, #ffffff 35%, transparent 70%)";
    document.body.appendChild(ripple);
    setTimeout(() => ripple.remove(), 700);
    update({ theme: next });
  };

  return (
    <div className="settings">
      <nav className="tabs">
        <h1>Swipelate</h1>
        <div className="tab-list">
          {(
            [
              ["general", t.tab_general],
              ["engine", t.tab_engine],
              ["subtitles", t.tab_subtitles],
              ["vocab", t.tab_vocab],
              ["history", t.tab_history],
              ["about", t.tab_about],
            ] as [Tab, string][]
          ).map(([id, name]) => (
            <button key={id} className={tab === id ? "tab active" : "tab"} onClick={() => setTab(id)}>
              {TAB_ICONS[id]}
              <span>{name}</span>
            </button>
          ))}
        </div>
        <div className="tab-bottom">
          <button
            className="theme-toggle"
            onClick={() => update({ app_language: appLang === "en" ? "vi" : "en" })}
            title={t.lang_toggle_title(appLang)}
          >
            <IconGlobe size={15} />
            <span>{appLang === "en" ? "English" : "Tiếng Việt"}</span>
          </button>
          <button
            className="theme-toggle"
            onClick={toggleTheme}
            title={t.theme_toggle_title(isDark)}
          >
            {isDark ? <IconMoon size={15} /> : <IconSun size={15} />}
            <span>{isDark ? t.theme_dark : t.theme_light}</span>
          </button>
          <p className="status">{status}</p>
        </div>
      </nav>

      <main className="pane">
        {updatedFrom !== null && (
          <button className="updated" title={appLang === "en" ? "Click to dismiss" : "Bấm để ẩn"} onClick={() => setUpdatedFrom(null)}>
            {t.app_updated_notice(updatedFrom, version)}
          </button>
        )}
        {tab === "general" && (
          <>
            <section>
              <h2>{t.usage_title}</h2>
              <p className="help">{t.usage_desc}</p>
            </section>

            <section>
              <h2>{t.trigger_title}</h2>
              <label className="row">
                <span>{t.hotkey_selection}</span>
                <input
                  className="hotkey"
                  value={config.hotkey}
                  readOnly
                  onKeyDown={(event) => {
                    event.preventDefault();
                    const shortcut = shortcutFrom(event);
                    if (shortcut) update({ hotkey: shortcut });
                  }}
                  title={t.hotkey_hint}
                />
              </label>
              <label className="row">
                <span>{t.hotkey_ocr}</span>
                <input
                  className="hotkey"
                  value={config.ocr_hotkey}
                  readOnly
                  onKeyDown={(event) => {
                    event.preventDefault();
                    const shortcut = shortcutFrom(event);
                    if (shortcut) update({ ocr_hotkey: shortcut });
                  }}
                  title={t.hotkey_hint}
                />
              </label>
              <label className="row">
                <span>{t.auto_trigger}</span>
                <input type="checkbox" checked={config.auto_trigger} onChange={(e) => update({ auto_trigger: e.target.checked })} />
              </label>
              <label className="row">
                <span>{t.instant_translate}</span>
                <input
                  type="checkbox"
                  checked={config.instant_translate}
                  disabled={!config.auto_trigger}
                  onChange={(e) => update({ instant_translate: e.target.checked })}
                />
              </label>
              <label className="row">
                <span>{t.hover_translate}</span>
                <input type="checkbox" checked={config.hover_translate} onChange={(e) => update({ hover_translate: e.target.checked })} />
              </label>
              <label className="row">
                <span>{t.hover_sentence}</span>
                <input
                  type="checkbox"
                  checked={config.hover_sentence}
                  disabled={!config.hover_translate}
                  onChange={(e) => update({ hover_sentence: e.target.checked })}
                />
              </label>
              <label className="row">
                <span>{t.listen_hide_captions}</span>
                <input type="checkbox" checked={config.listen_hide_captions} onChange={(e) => update({ listen_hide_captions: e.target.checked })} />
              </label>
              <label className="row">
                <span>{t.explain_in_context}</span>
                <input type="checkbox" checked={config.explain_in_context} onChange={(e) => update({ explain_in_context: e.target.checked })} />
              </label>
              <p className="help">{t.explain_help}</p>
              <label className="row">
                <span>{t.window_hotkey}</span>
                <input
                  className="hotkey"
                  title={t.hotkey_hint}
                  readOnly
                  value={config.window_hotkey}
                  onKeyDown={(e) => {
                    e.preventDefault();
                    const shortcut = shortcutFrom(e);
                    if (shortcut) update({ window_hotkey: shortcut });
                  }}
                />
              </label>
              <label className="row">
                <span>{t.live_hotkey}</span>
                <input
                  className="hotkey"
                  title={t.hotkey_hint}
                  readOnly
                  value={config.live_hotkey}
                  onKeyDown={(e) => {
                    e.preventDefault();
                    const shortcut = shortcutFrom(e);
                    if (shortcut) update({ live_hotkey: shortcut });
                  }}
                />
              </label>
              <label className="row">
                <span>{t.clipboard_hotkey}</span>
                <input
                  className="hotkey"
                  title={t.clipboard_hotkey_hint}
                  readOnly
                  placeholder={t.clipboard_off}
                  value={config.clipboard_hotkey}
                  onKeyDown={(e) => {
                    e.preventDefault();
                    if (e.key === "Backspace" || e.key === "Delete") return update({ clipboard_hotkey: "" });
                    const shortcut = shortcutFrom(e);
                    if (shortcut) update({ clipboard_hotkey: shortcut });
                  }}
                />
              </label>
              <label className="row">
                <span>{t.popup_size(config.popup_font_size)}</span>
                <input
                  type="range"
                  min={12}
                  max={22}
                  value={config.popup_font_size}
                  onChange={(e) => update({ popup_font_size: Number(e.target.value) })}
                />
              </label>
              <label className="row">
                <span>{t.popup_opacity(config.popup_opacity)}</span>
                <input
                  type="range"
                  min={60}
                  max={100}
                  step={5}
                  value={config.popup_opacity}
                  onChange={(e) => update({ popup_opacity: Number(e.target.value) })}
                />
              </label>
              <label className="row">
                <span>{t.autostart}</span>
                <input type="checkbox" checked={autostart} onChange={(e) => toggleAutostart(e.target.checked)} />
              </label>
              <label className="row">
                <span>{t.theme}</span>
                <select value={config.theme} onChange={(e) => update({ theme: e.target.value as Config["theme"] })}>
                  <option value="system">{t.theme_system}</option>
                  <option value="light">{t.theme_light_opt}</option>
                  <option value="dark">{t.theme_dark_opt}</option>
                </select>
              </label>
              <label className="row">
                <span>{t.app_language}</span>
                <select
                  value={config.app_language || "vi"}
                  onChange={(e) => update({ app_language: e.target.value as AppLanguage })}
                >
                  <option value="vi">Tiếng Việt</option>
                  <option value="en">English</option>
                </select>
              </label>
              <label className="row">
                <span>{t.stats_label}</span>
                <input type="checkbox" checked={config.usage_stats} onChange={(e) => update({ usage_stats: e.target.checked })} />
              </label>
              <p className="help">{t.stats_help}</p>
            </section>

            <section>
              <h2>{t.lang_section}</h2>
              <label className="row">
                <span>{t.translate_to}</span>
                <select value={config.primary_lang} onChange={(e) => update({ primary_lang: e.target.value })}>
                  {LANGUAGES.map(([code]) => (
                    <option key={code} value={code}>
                      {langName(code, appLang)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="row">
                <span>{t.secondary_to(langName(config.primary_lang, appLang))}</span>
                <select value={config.secondary_lang} onChange={(e) => update({ secondary_lang: e.target.value })}>
                  {LANGUAGES.filter(([code]) => code !== config.primary_lang).map(([code]) => (
                    <option key={code} value={code}>
                      {langName(code, appLang)}
                    </option>
                  ))}
                </select>
              </label>
            </section>

            <section>
              <h2>{t.excluded_title}</h2>
              <p className="help">{t.excluded_help}</p>
              <textarea
                rows={3}
                defaultValue={config.excluded_apps.join("\n")}
                onBlur={(e) =>
                  update({
                    excluded_apps: e.target.value
                      .split("\n")
                      .map((line) => line.trim().toLowerCase())
                      .filter(Boolean),
                  })
                }
              />
            </section>

            <section>
              <h2>{t.profiles_title}</h2>
              <p className="help">{t.profiles_help}</p>
              {config.profiles.map((profile, index) => {
                const change = (patch: Partial<Profile>) =>
                  update({ profiles: config.profiles.map((item, at) => (at === index ? { ...item, ...patch } : item)) });
                return (
                  <div className="profile" key={index}>
                    <input
                      aria-label="Tên file chạy"
                      placeholder="chrome.exe"
                      defaultValue={profile.app}
                      onBlur={(e) => change({ app: e.target.value.trim().toLowerCase() })}
                    />
                    <select aria-label="Nguồn dịch" value={profile.engine} onChange={(e) => change({ engine: e.target.value })}>
                      <option value="">{t.default_engine}</option>
                      {Object.entries(ENGINE_NAMES).map(([id, name]) => (
                        <option key={id} value={id}>
                          {name}
                        </option>
                      ))}
                    </select>
                    <select
                      aria-label="Dịch sang"
                      value={profile.primary_lang}
                      onChange={(e) => change({ primary_lang: e.target.value })}
                    >
                      <option value="">{t.default_lang}</option>
                      {LANGUAGES.map(([code]) => (
                        <option key={code} value={code}>
                          {langName(code, appLang)}
                        </option>
                      ))}
                    </select>
                    <button onClick={() => update({ profiles: config.profiles.filter((_, at) => at !== index) })}>{t.remove_profile}</button>
                  </div>
                );
              })}
              <button
                onClick={async () => {
                  // Start from the application the user was just in, when known.
                  const app = (await invoke<string | null>("last_app")) ?? "";
                  const known = config.profiles.some((profile) => profile.app === app);
                  update({ profiles: [...config.profiles, { app: known ? "" : app, engine: "", primary_lang: "" }] });
                }}
              >
                {t.add_profile}
              </button>
            </section>
          </>
        )}

        {tab === "engine" && (
          <>
            <section>
              <h2>{t.engine_default_title}</h2>
              {getEngines(appLang).map((engine) => (
                <label key={engine.id} className="choice">
                  <input type="radio" name="engine" checked={config.engine === engine.id} onChange={() => update({ engine: engine.id })} />
                  <span>
                    <strong>{ENGINE_NAMES[engine.id]}</strong>
                    <small>{engine.hint}</small>
                  </span>
                </label>
              ))}
              <label className="row">
                <span>{t.fallback_offline}</span>
                <input type="checkbox" checked={config.fallback_offline} onChange={(e) => update({ fallback_offline: e.target.checked })} />
              </label>
            </section>

            <section>
              <h2>{t.api_keys_title}</h2>
              <p className="help">{t.api_keys_help}</p>
              <label className="row">
                <span>DeepL</span>
                <input type="password" defaultValue={config.deepl_key} onBlur={(e) => update({ deepl_key: e.target.value.trim() })} />
              </label>
              <label className="row">
                <span>Anthropic (Claude)</span>
                <input type="password" defaultValue={config.anthropic_key} onBlur={(e) => update({ anthropic_key: e.target.value.trim() })} />
              </label>
            </section>

            <section>
              <h2>{t.dict_sources}</h2>
              <p className="help">{t.dict_help}</p>
              <label className="row">
                <span>{t.english_definitions}</span>
                <input type="checkbox" checked={config.english_definitions} onChange={(e) => update({ english_definitions: e.target.checked })} />
              </label>
            </section>

            <section>
              <h2>{t.openai_title}</h2>
              <p className="help">{t.openai_help}</p>
              <label className="row">
                <span>{t.api_url}</span>
                <input defaultValue={config.openai_base} onBlur={(e) => update({ openai_base: e.target.value.trim() })} />
              </label>
              <label className="row">
                <span>{t.model_name}</span>
                <input defaultValue={config.openai_model} onBlur={(e) => update({ openai_model: e.target.value.trim() })} />
              </label>
              <label className="row">
                <span>{t.api_key}</span>
                <input type="password" defaultValue={config.openai_key} onBlur={(e) => update({ openai_key: e.target.value.trim() })} />
              </label>
            </section>

            <section>
              <h2>{t.oxford_title}</h2>
              <label className="row">
                <span>App ID</span>
                <input type="password" defaultValue={config.oxford_app_id} onBlur={(e) => update({ oxford_app_id: e.target.value.trim() })} />
              </label>
              <label className="row">
                <span>App Key</span>
                <input type="password" defaultValue={config.oxford_app_key} onBlur={(e) => update({ oxford_app_key: e.target.value.trim() })} />
              </label>
            </section>

            <OfflineModels lang={appLang} />

            <section>
              <h2>{t.test_title}</h2>
              <textarea rows={3} value={sample} onChange={(e) => setSample(e.target.value)} />
              <button className="primary" onClick={runSample} disabled={busy || !sample.trim()}>
                {busy ? t.test_busy : t.test_btn}
              </button>
              {sampleResult && (
                <p className="sample">
                  <small>
                    {sampleResult.from.toUpperCase()} → {sampleResult.to.toUpperCase()} · {ENGINE_NAMES[sampleResult.engine] ?? sampleResult.engine}
                  </small>
                  {sampleResult.text}
                  {sampleResult.note && <small>{sampleResult.note}</small>}
                </p>
              )}
              {sampleError && <p className="error">{sampleError}</p>}
            </section>
          </>
        )}

        {tab === "history" && (
          <section>
            <h2>{t.history_title}</h2>
            <label className="row">
              <span>{t.save_history}</span>
              <input type="checkbox" checked={config.save_history} onChange={(e) => update({ save_history: e.target.checked })} />
            </label>
            {config.save_history && (
              <>
                <label className="row">
                  <span>{t.retention_days}</span>
                  <select
                    value={config.history_retention_days ?? 30}
                    onChange={(e) => update({ history_retention_days: Number(e.target.value) })}
                  >
                    <option value={7}>{t.days_7}</option>
                    <option value={14}>{t.days_14}</option>
                    <option value={30}>{t.days_30}</option>
                    <option value={60}>{t.days_60}</option>
                    <option value={90}>{t.days_90}</option>
                    <option value={180}>{t.days_180}</option>
                    <option value={0}>{t.days_forever}</option>
                  </select>
                </label>
                <label className="row">
                  <span>{t.max_entries}</span>
                  <select
                    value={config.history_max_entries ?? 500}
                    onChange={(e) => update({ history_max_entries: Number(e.target.value) })}
                  >
                    <option value={100}>{t.entries_100}</option>
                    <option value={300}>{t.entries_300}</option>
                    <option value={500}>{t.entries_500}</option>
                    <option value={1000}>{t.entries_1000}</option>
                    <option value={2000}>{t.entries_2000}</option>
                    <option value={0}>{t.entries_unlimited}</option>
                  </select>
                </label>
                <p className="help">{t.history_help}</p>
              </>
            )}
            {history.length === 0 ? (
              <p className="help">{t.no_history}</p>
            ) : (
              <>
                <div className="row-buttons">
                  <button
                    onClick={async () => {
                      const count = await invoke<number>("history_clear_older_than", { days: 30 });
                      setStatus(appLang === "en" ? `Cleaned ${count} entries older than 30 days.` : `Đã dọn dẹp ${count} mục cũ hơn 30 ngày.`);
                      setHistory(await invoke<HistoryEntry[]>("history_list"));
                    }}
                  >
                    {t.clear_older_30}
                  </button>
                  {historyNeedle && (
                    <button
                      className="danger"
                      onClick={async () => {
                        const count = await invoke<number>("history_clear_matches", { query: historyQuery.trim() });
                        setStatus(appLang === "en" ? `Deleted ${count} search matches.` : `Đã xoá ${count} mục tìm kiếm.`);
                        setHistory(await invoke<HistoryEntry[]>("history_list"));
                      }}
                    >
                      {t.clear_filtered(filteredHistory.length)}
                    </button>
                  )}
                  <button
                    className="danger"
                    onClick={async () => {
                      await invoke("history_clear");
                      setHistory([]);
                      setStatus(appLang === "en" ? "Cleared all history." : "Đã xoá toàn bộ lịch sử.");
                    }}
                  >
                    {t.clear_all_history(history.length)}
                  </button>
                </div>
                <input
                  className="search"
                  type="search"
                  placeholder={t.search_history_ph(history.length)}
                  value={historyQuery}
                  onChange={(e) => {
                    setHistoryQuery(e.target.value);
                    setHistoryVisibleCount(40);
                  }}
                />
                {filteredHistory.length === 0 ? (
                  <p className="help">{t.no_history_matches}</p>
                ) : (
                  <>
                    <p className="help count-hint">
                      {t.showing_history(Math.min(historyVisibleCount, filteredHistory.length), filteredHistory.length)}
                    </p>
                    <ul className="history">
                      {filteredHistory.slice(0, historyVisibleCount).map((entry) => (
                        <li key={`${entry.at}-${entry.source}`}>
                          <small>
                            {new Date(entry.at * 1000).toLocaleString(appLang === "en" ? "en-US" : "vi-VN")} · {entry.from.toUpperCase()} → {entry.to.toUpperCase()} ·{" "}
                            {ENGINE_NAMES[entry.engine] ?? entry.engine}
                          </small>
                          <p className="from">{entry.source}</p>
                          <p>{entry.text}</p>
                        </li>
                      ))}
                    </ul>
                    {filteredHistory.length > historyVisibleCount && (
                      <div className="pagination">
                        <button onClick={() => setHistoryVisibleCount((c) => c + 40)}>
                          {t.load_more(40, filteredHistory.length - historyVisibleCount)}
                        </button>
                      </div>
                    )}
                  </>
                )}
              </>
            )}
          </section>
        )}

        {tab === "subtitles" && <Subtitles lang={appLang} />}

        {tab === "vocab" && <Vocab lang={appLang} />}

        {tab === "about" && (
          <About lang={appLang} checkUpdates={config.check_updates} setCheckUpdates={(on) => update({ check_updates: on })} />
        )}
      </main>
    </div>
  );
}
