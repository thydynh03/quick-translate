import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ENGINE_NAMES, HistoryEntry, UpdateInfo, LANGUAGES, Profile, Settings as Config, Translation, langName } from "./types";
import OfflineModels from "./OfflineModels";
import Subtitles from "./Subtitles";
import Vocab from "./Vocab";
import { IconAbout, IconEngine, IconGeneral, IconHistory, IconMoon, IconSubtitles, IconSun, IconVocab } from "./icons";

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

function About({ checkUpdates, setCheckUpdates }: { checkUpdates: boolean; setCheckUpdates: (on: boolean) => void }) {
  const [version, setVersion] = useState("");
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  // Installed from the Microsoft Store, which does the updating.
  const [store, setStore] = useState(false);

  const check = async () => {
    setBusy(true);
    setStatus("Đang kiểm tra…");
    try {
      const info = await invoke<UpdateInfo>("update_check");
      setUpdate(info);
      setStatus(info.available ? "" : `Bạn đang dùng bản mới nhất (${info.current}).`);
    } catch (error) {
      setStatus(`Không kiểm tra được: ${error}`);
    } finally {
      setBusy(false);
    }
  };

  const install = async () => {
    setBusy(true);
    setProgress(0);
    setStatus("Đang tải bộ cài…");
    try {
      await invoke("update_install");
      // Windows quits here to run the installer; macOS opens the disk image.
      setStatus("Đã mở bộ cài. Kéo Swipelate vào Applications để thay bản cũ, rồi mở lại app. Còn Quick Translate (tên cũ) trong Applications thì xoá đi.");
    } catch (error) {
      setStatus(`Không cập nhật được: ${error}`);
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
      <h2>Giới thiệu</h2>
      <p>Swipelate {version && `phiên bản ${version}`}</p>

      {store ? (
        <p className="help">Bản cài từ Microsoft Store: Store tự cập nhật Swipelate.</p>
      ) : (
        <div className="update">
          {update?.available ? (
            <>
              <p>
                <strong>Có bản mới {update.latest}</strong>
                {update.published && <small> · {new Date(update.published).toLocaleDateString("vi-VN")}</small>}
              </p>
              {update.notes && <pre className="notes">{update.notes}</pre>}
              <div className="actions">
                <button className="primary" onClick={install} disabled={busy}>
                  {progress !== null ? `Đang tải ${progress}%` : "Cập nhật ngay"}
                </button>
                <button onClick={() => openUrl(update.page)}>Xem trên GitHub</button>
              </div>
              <p className="help">Bộ cài được kiểm tra mã SHA-256 trước khi chạy. Trên Windows app tự đóng, cài bản mới rồi mở lại.</p>
            </>
          ) : (
            <div className="actions">
              <button onClick={check} disabled={busy}>
                Kiểm tra cập nhật
              </button>
            </div>
          )}
          {status && <p className="help">{status}</p>}
          <label className="row">
            <span>Tự kiểm tra bản mới khi mở app và mỗi ngày</span>
            <input type="checkbox" checked={checkUpdates} onChange={(e) => setCheckUpdates(e.target.checked)} />
          </label>
        </div>
      )}

      <p className="help">
        Bản quyền © 2026 thydynh03. Phần mềm phát hành theo giấy phép MIT: bạn được dùng, sao chép và sửa đổi miễn phí, kèm
        theo thông báo bản quyền. Phần mềm được cung cấp nguyên trạng, không có bảo hành.
      </p>
      <p className="help">
        Danh sách từ Anh–Việt lấy từ Wiktionary (CC BY-SA 3.0). Model dịch offline Opus-MT của Helsinki-NLP (CC BY 4.0) được
        tải về khi bạn yêu cầu.
      </p>
      <div className="actions">
        <button onClick={() => openUrl(`${SITE}docs.html`)}>Hướng dẫn sử dụng</button>
        <button onClick={() => openUrl(`${SITE}license.html`)}>Giấy phép</button>
        <button onClick={() => openUrl(`${SITE}privacy.html`)}>Quyền riêng tư</button>
        <button onClick={() => openUrl(`${REPO}/issues`)}>Báo lỗi</button>
      </div>
    </section>
  );
}

const ENGINES: { id: string; hint: string }[] = [
  { id: "google", hint: "Không cần key. Dùng endpoint web không chính thức, có thể bị giới hạn." },
  { id: "deepl", hint: "Cần API key của DeepL." },
  { id: "claude", hint: "Cần API key của Anthropic. Dịch tự nhiên hơn với câu dài, chậm hơn." },
  { id: "offline", hint: "Chạy trên máy, không cần mạng. Cần tải model trước." },
  { id: "openai", hint: "ChatGPT, Gemini hoặc model chạy trên máy (Ollama, LM Studio). Điền địa chỉ API và model bên dưới." },
];

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

  const historyNeedle = historyQuery.trim().toLowerCase();
  const filteredHistory = history.filter((entry) => `${entry.source}\n${entry.text}`.toLowerCase().includes(historyNeedle));

  const update = async (patch: Partial<Config>) => {
    const next = { ...config, ...patch };
    setConfig(next);
    try {
      const failed = await invoke<string[]>("save_settings", { settings: next });
      setStatus(failed.length ? `Phím tắt ${failed.join(", ")} đang bị ứng dụng khác dùng.` : "Đã lưu");
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
              ["general", "Chung"],
              ["engine", "Engine dịch"],
              ["subtitles", "Phụ đề"],
              ["vocab", "Từ vựng"],
              ["history", "Lịch sử"],
              ["about", "Giới thiệu"],
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
            onClick={toggleTheme}
            title={`Chế độ hiện tại: ${isDark ? "Tối" : "Sáng"}. Bấm để chuyển đổi giao diện`}
          >
            {isDark ? <IconMoon size={15} /> : <IconSun size={15} />}
            <span>{isDark ? "Chế độ tối" : "Chế độ sáng"}</span>
          </button>
          <p className="status">{status}</p>
        </div>
      </nav>

      <main className="pane">
        {updatedFrom !== null && (
          <button className="updated" title="Bấm để ẩn" onClick={() => setUpdatedFrom(null)}>
            Đã cập nhật{updatedFrom ? ` từ ${updatedFrom}` : ""} lên phiên bản {version}. Ứng dụng đang chạy ở khay hệ thống.
          </button>
        )}
        {tab === "general" && (
          <>
            <section>
              <h2>Cách dùng</h2>
              <p className="help">
                Bôi đen văn bản ở bất kỳ ứng dụng nào rồi bấm vào biểu tượng nhỏ hiện cạnh con trỏ, hoặc nhấn phím tắt.
                Double-click để dịch một từ, triple-click để dịch một câu.
              </p>
            </section>

            <section>
              <h2>Kích hoạt</h2>
              <label className="row">
                <span>Phím tắt dịch vùng bôi đen</span>
                <input
                  className="hotkey"
                  value={config.hotkey}
                  readOnly
                  onKeyDown={(event) => {
                    event.preventDefault();
                    const shortcut = shortcutFrom(event);
                    if (shortcut) update({ hotkey: shortcut });
                  }}
                  title="Bấm vào rồi nhấn tổ hợp phím mới"
                />
              </label>
              <label className="row">
                <span>Phím tắt dịch vùng màn hình (OCR)</span>
                <input
                  className="hotkey"
                  value={config.ocr_hotkey}
                  readOnly
                  onKeyDown={(event) => {
                    event.preventDefault();
                    const shortcut = shortcutFrom(event);
                    if (shortcut) update({ ocr_hotkey: shortcut });
                  }}
                  title="Bấm vào rồi nhấn tổ hợp phím mới"
                />
              </label>
              <label className="row">
                <span>Tự hiện biểu tượng khi bôi đen</span>
                <input type="checkbox" checked={config.auto_trigger} onChange={(e) => update({ auto_trigger: e.target.checked })} />
              </label>
              <label className="row">
                <span>Bôi đen là dịch ngay, không cần bấm biểu tượng hay phím tắt</span>
                <input
                  type="checkbox"
                  checked={config.instant_translate}
                  disabled={!config.auto_trigger}
                  onChange={(e) => update({ instant_translate: e.target.checked })}
                />
              </label>
              <label className="row">
                <span>Giữ Ctrl và rê chuột lên văn bản để dịch (thử nghiệm, chỉ ở ứng dụng hỗ trợ)</span>
                <input type="checkbox" checked={config.hover_translate} onChange={(e) => update({ hover_translate: e.target.checked })} />
              </label>
              <label className="row">
                <span>Khi rê chuột: dịch cả câu (từ đầu câu đến dấu chấm) thay vì một từ</span>
                <input
                  type="checkbox"
                  checked={config.hover_sentence}
                  disabled={!config.hover_translate}
                  onChange={(e) => update({ hover_sentence: e.target.checked })}
                />
              </label>
              <label className="row">
                <span>Nghe tiếng: ẩn phụ đề tiếng Anh của Windows, chỉ hiện bản dịch ở đáy màn hình</span>
                <input type="checkbox" checked={config.listen_hide_captions} onChange={(e) => update({ listen_hide_captions: e.target.checked })} />
              </label>
              <label className="row">
                <span>Bôi đen 1–4 từ: giải thích nghĩa theo ngữ cảnh câu chứa chúng</span>
                <input type="checkbox" checked={config.explain_in_context} onChange={(e) => update({ explain_in_context: e.target.checked })} />
              </label>
              <p className="help">
                Có Claude hoặc API tương thích OpenAI (tab Engine dịch) thì app giải thích nghĩa của từ trong câu đó. Không có thì app
                dịch cả câu để bạn thấy ngữ cảnh.
              </p>
              <label className="row">
                <span>Phím tắt dịch cả cửa sổ đang mở</span>
                <input
                  className="hotkey"
                  title="Bấm vào rồi nhấn tổ hợp phím mới"
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
                <span>Phím tắt bật/tắt dịch thời gian thực</span>
                <input
                  className="hotkey"
                  title="Bấm vào rồi nhấn tổ hợp phím mới"
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
                <span>Phím tắt bảng clipboard (Backspace để tắt)</span>
                <input
                  className="hotkey"
                  title="Bấm vào rồi nhấn tổ hợp phím mới; Backspace để tắt bảng clipboard"
                  readOnly
                  placeholder="Đang tắt"
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
                <span>Cỡ chữ popup ({config.popup_font_size}px)</span>
                <input
                  type="range"
                  min={12}
                  max={22}
                  value={config.popup_font_size}
                  onChange={(e) => update({ popup_font_size: Number(e.target.value) })}
                />
              </label>
              <label className="row">
                <span>Độ đậm nền popup ({config.popup_opacity}%)</span>
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
                <span>Khởi động cùng máy</span>
                <input type="checkbox" checked={autostart} onChange={(e) => toggleAutostart(e.target.checked)} />
              </label>
              <label className="row">
                <span>Giao diện</span>
                <select value={config.theme} onChange={(e) => update({ theme: e.target.value as Config["theme"] })}>
                  <option value="system">Theo hệ thống</option>
                  <option value="light">Sáng</option>
                  <option value="dark">Tối</option>
                </select>
              </label>
              <label className="row">
                <span>Gửi thống kê ẩn danh (đếm số người dùng)</span>
                <input type="checkbox" checked={config.usage_stats} onChange={(e) => update({ usage_stats: e.target.checked })} />
              </label>
              <p className="help">Mỗi ngày một lần: mã ngẫu nhiên của máy, phiên bản, hệ điều hành. Không gửi nội dung dịch.</p>
            </section>

            <section>
              <h2>Ngôn ngữ</h2>
              <label className="row">
                <span>Dịch sang</span>
                <select value={config.primary_lang} onChange={(e) => update({ primary_lang: e.target.value })}>
                  {LANGUAGES.map(([code, name]) => (
                    <option key={code} value={code}>
                      {name}
                    </option>
                  ))}
                </select>
              </label>
              <label className="row">
                <span>Khi văn bản đã là {langName(config.primary_lang)}, dịch sang</span>
                <select value={config.secondary_lang} onChange={(e) => update({ secondary_lang: e.target.value })}>
                  {LANGUAGES.filter(([code]) => code !== config.primary_lang).map(([code, name]) => (
                    <option key={code} value={code}>
                      {name}
                    </option>
                  ))}
                </select>
              </label>
            </section>

            <section>
              <h2>Ứng dụng không tự hiện biểu tượng</h2>
              <p className="help">Mỗi dòng một tên file chạy, ví dụ: photoshop.exe</p>
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
              <h2>Hồ sơ theo ứng dụng</h2>
              <p className="help">
                Dùng nguồn dịch hoặc ngôn ngữ đích khác khi một ứng dụng đang mở, ví dụ dịch offline trong game. Nhập tên file chạy
                như chrome.exe; ô nào để “Mặc định” thì theo cài đặt chung.
              </p>
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
                      <option value="">Nguồn dịch: mặc định</option>
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
                      <option value="">Dịch sang: mặc định</option>
                      {LANGUAGES.map(([code, name]) => (
                        <option key={code} value={code}>
                          {name}
                        </option>
                      ))}
                    </select>
                    <button onClick={() => update({ profiles: config.profiles.filter((_, at) => at !== index) })}>Xoá</button>
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
                Thêm hồ sơ
              </button>
            </section>
          </>
        )}

        {tab === "engine" && (
          <>
            <section>
              <h2>Engine mặc định</h2>
              {ENGINES.map((engine) => (
                <label key={engine.id} className="choice">
                  <input type="radio" name="engine" checked={config.engine === engine.id} onChange={() => update({ engine: engine.id })} />
                  <span>
                    <strong>{ENGINE_NAMES[engine.id]}</strong>
                    <small>{engine.hint}</small>
                  </span>
                </label>
              ))}
              <label className="row">
                <span>Khi engine chính lỗi, tự dùng dự phòng: offline trước, rồi API có key</span>
                <input type="checkbox" checked={config.fallback_offline} onChange={(e) => update({ fallback_offline: e.target.checked })} />
              </label>
            </section>

            <section>
              <h2>API key</h2>
              <p className="help">Key được lưu dạng văn bản thường trong thư mục dữ liệu của ứng dụng trên máy này.</p>
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
              <h2>Nguồn từ điển</h2>
              <p className="help">
                Khi tra một từ tiếng Anh, popup có nút mở từ đó trong từ điển Cambridge và Oxford trên trình duyệt. Nếu bạn có tài khoản Oxford
                Dictionaries API, nhập App ID và App Key để lấy định nghĩa Oxford ngay trong popup; không có thì dùng Free Dictionary API.
              </p>
              <label className="row">
                <span>Thêm định nghĩa tiếng Anh và phiên âm IPA (Free Dictionary API, cần mạng)</span>
                <input type="checkbox" checked={config.english_definitions} onChange={(e) => update({ english_definitions: e.target.checked })} />
              </label>
            </section>

            <section>
              <h2>AI qua OpenAI API (tuỳ chọn)</h2>
              <p className="help">
                Dùng được với mọi dịch vụ tương thích OpenAI. ChatGPT: https://api.openai.com/v1. Gemini:
                https://generativelanguage.googleapis.com/v1beta/openai. Ollama trên máy: http://localhost:11434/v1 (không cần key).
              </p>
              <label className="row">
                <span>Địa chỉ API</span>
                <input defaultValue={config.openai_base} onBlur={(e) => update({ openai_base: e.target.value.trim() })} />
              </label>
              <label className="row">
                <span>Tên model</span>
                <input defaultValue={config.openai_model} onBlur={(e) => update({ openai_model: e.target.value.trim() })} />
              </label>
              <label className="row">
                <span>API key</span>
                <input type="password" defaultValue={config.openai_key} onBlur={(e) => update({ openai_key: e.target.value.trim() })} />
              </label>
            </section>

            <section>
              <h2>Oxford Dictionaries API (tuỳ chọn)</h2>
              <label className="row">
                <span>App ID</span>
                <input type="password" defaultValue={config.oxford_app_id} onBlur={(e) => update({ oxford_app_id: e.target.value.trim() })} />
              </label>
              <label className="row">
                <span>App Key</span>
                <input type="password" defaultValue={config.oxford_app_key} onBlur={(e) => update({ oxford_app_key: e.target.value.trim() })} />
              </label>
            </section>

            <OfflineModels />

            <section>
              <h2>Dịch thử</h2>
              <textarea rows={3} value={sample} onChange={(e) => setSample(e.target.value)} />
              <button className="primary" onClick={runSample} disabled={busy || !sample.trim()}>
                {busy ? "Đang dịch…" : "Dịch thử"}
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
            <h2>Lịch sử</h2>
            <label className="row">
              <span>Lưu lịch sử dịch</span>
              <input type="checkbox" checked={config.save_history} onChange={(e) => update({ save_history: e.target.checked })} />
            </label>
            {config.save_history && (
              <>
                <label className="row">
                  <span>Tự động xoá sau thời gian</span>
                  <select
                    value={config.history_retention_days ?? 30}
                    onChange={(e) => update({ history_retention_days: Number(e.target.value) })}
                  >
                    <option value={7}>7 ngày</option>
                    <option value={14}>14 ngày</option>
                    <option value={30}>30 ngày (Mặc định)</option>
                    <option value={60}>60 ngày</option>
                    <option value={90}>90 ngày</option>
                    <option value={180}>180 ngày</option>
                    <option value={0}>Không bao giờ (Lưu mãi mãi)</option>
                  </select>
                </label>
                <label className="row">
                  <span>Số lượng bản dịch tối đa</span>
                  <select
                    value={config.history_max_entries ?? 500}
                    onChange={(e) => update({ history_max_entries: Number(e.target.value) })}
                  >
                    <option value={100}>100 mục</option>
                    <option value={300}>300 mục</option>
                    <option value={500}>500 mục (Mặc định)</option>
                    <option value={1000}>1.000 mục</option>
                    <option value={2000}>2.000 mục</option>
                    <option value={0}>Không giới hạn số lượng</option>
                  </select>
                </label>
                <p className="help">
                  Lịch sử được tự động dọn dẹp theo các giới hạn trên khi mở ứng dụng và sau mỗi bản dịch mới để ứng dụng luôn nhanh và nhẹ.
                </p>
              </>
            )}
            {history.length === 0 ? (
              <p className="help">Chưa có bản dịch nào.</p>
            ) : (
              <>
                <div className="row-buttons">
                  <button
                    onClick={async () => {
                      const count = await invoke<number>("history_clear_older_than", { days: 30 });
                      setStatus(`Đã dọn dẹp ${count} mục cũ hơn 30 ngày.`);
                      setHistory(await invoke<HistoryEntry[]>("history_list"));
                    }}
                  >
                    Dọn mục &gt; 30 ngày
                  </button>
                  {historyNeedle && (
                    <button
                      className="danger"
                      onClick={async () => {
                        const count = await invoke<number>("history_clear_matches", { query: historyQuery.trim() });
                        setStatus(`Đã xoá ${count} mục tìm kiếm.`);
                        setHistory(await invoke<HistoryEntry[]>("history_list"));
                      }}
                    >
                      Xoá {filteredHistory.length} kết quả tìm kiếm
                    </button>
                  )}
                  <button
                    className="danger"
                    onClick={async () => {
                      await invoke("history_clear");
                      setHistory([]);
                      setStatus("Đã xoá toàn bộ lịch sử.");
                    }}
                  >
                    Xoá toàn bộ ({history.length})
                  </button>
                </div>
                <input
                  className="search"
                  type="search"
                  placeholder={`Tìm trong ${history.length} bản dịch…`}
                  value={historyQuery}
                  onChange={(e) => {
                    setHistoryQuery(e.target.value);
                    setHistoryVisibleCount(40);
                  }}
                />
                {filteredHistory.length === 0 ? (
                  <p className="help">Không có mục nào khớp.</p>
                ) : (
                  <>
                    <p className="help count-hint">
                      Đang hiển thị {Math.min(historyVisibleCount, filteredHistory.length)} trên {filteredHistory.length} mục
                    </p>
                    <ul className="history">
                      {filteredHistory.slice(0, historyVisibleCount).map((entry) => (
                        <li key={`${entry.at}-${entry.source}`}>
                          <small>
                            {new Date(entry.at * 1000).toLocaleString("vi-VN")} · {entry.from.toUpperCase()} → {entry.to.toUpperCase()} ·{" "}
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
                          Tải thêm 40 mục ({filteredHistory.length - historyVisibleCount} mục còn lại)
                        </button>
                      </div>
                    )}
                  </>
                )}
              </>
            )}
          </section>
        )}

        {tab === "subtitles" && <Subtitles />}

        {tab === "vocab" && <Vocab />}

        {tab === "about" && <About checkUpdates={config.check_updates} setCheckUpdates={(on) => update({ check_updates: on })} />}
      </main>
    </div>
  );
}
