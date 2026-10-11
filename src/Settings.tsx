import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ENGINE_NAMES, HistoryEntry, LANGUAGES, Settings as Config, Translation, langName } from "./types";
import OfflineModels from "./OfflineModels";

type Tab = "general" | "engine" | "history" | "about";

const SITE = "https://thydynh03.github.io/quick-translate/";
const REPO = "https://github.com/thydynh03/quick-translate";

function About() {
  const [version, setVersion] = useState("");
  useEffect(() => {
    getVersion().then(setVersion);
  }, []);
  return (
    <section>
      <h2>Giới thiệu</h2>
      <p>Quick Translate {version && `phiên bản ${version}`}</p>
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
        <button onClick={() => openUrl(`${REPO}/releases/latest`)}>Kiểm tra bản mới</button>
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
  const [config, setConfig] = useState<Config | null>(null);
  const [autostart, setAutostart] = useState(false);
  const [status, setStatus] = useState("");
  const [sample, setSample] = useState("The quick brown fox jumps over the lazy dog.");
  const [sampleResult, setSampleResult] = useState<Translation | null>(null);
  const [sampleError, setSampleError] = useState("");
  const [busy, setBusy] = useState(false);
  const [history, setHistory] = useState<HistoryEntry[]>([]);

  const load = () => invoke<Config>("get_settings").then(setConfig);

  useEffect(() => {
    load();
    isEnabled().then(setAutostart).catch(() => {});
    const unlisten = listen("settings-changed", load);
    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  useEffect(() => {
    if (tab === "history") invoke<HistoryEntry[]>("history_list").then(setHistory);
  }, [tab]);

  if (!config) return null;

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
      await (on ? enable() : disable());
      setAutostart(on);
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

  return (
    <div className="settings">
      <nav className="tabs">
        <h1>Quick Translate</h1>
        {(
          [
            ["general", "Chung"],
            ["engine", "Engine dịch"],
            ["history", "Lịch sử"],
            ["about", "Giới thiệu"],
          ] as [Tab, string][]
        ).map(([id, name]) => (
          <button key={id} className={tab === id ? "tab active" : "tab"} onClick={() => setTab(id)}>
            {name}
          </button>
        ))}
        <p className="status">{status}</p>
      </nav>

      <main className="pane">
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
                <span>Giữ Ctrl và rê chuột lên một từ để dịch (thử nghiệm, chỉ ở ứng dụng hỗ trợ)</span>
                <input type="checkbox" checked={config.hover_translate} onChange={(e) => update({ hover_translate: e.target.checked })} />
              </label>
              <label className="row">
                <span>Khởi động cùng máy</span>
                <input type="checkbox" checked={autostart} onChange={(e) => toggleAutostart(e.target.checked)} />
              </label>
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
            {history.length === 0 ? (
              <p className="help">Chưa có bản dịch nào.</p>
            ) : (
              <>
                <button
                  onClick={async () => {
                    await invoke("history_clear");
                    setHistory([]);
                  }}
                >
                  Xoá toàn bộ lịch sử
                </button>
                <ul className="history">
                  {history.map((entry) => (
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
              </>
            )}
          </section>
        )}

        {tab === "about" && <About />}
      </main>
    </div>
  );
}
