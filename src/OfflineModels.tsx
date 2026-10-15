import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { langName } from "./types";
import { AppLanguage, useT } from "./i18n";

type Model = {
  id: string;
  from: string;
  to: string;
  size_mb: number;
  installed: boolean;
};

type Progress = { id: string; done: number; total: number; error: string | null; finished: boolean };

export default function OfflineModels({ lang = "vi" }: { lang?: AppLanguage }) {
  const t = useT(lang);
  const [models, setModels] = useState<Model[]>([]);
  const [progress, setProgress] = useState<Record<string, Progress>>({});
  const [error, setError] = useState("");

  const load = () => invoke<Model[]>("offline_models").then(setModels).catch((e) => setError(String(e)));

  useEffect(() => {
    load();
    const unlisten = listen<Progress>("offline-progress", (event) => {
      const p = event.payload;
      setProgress((all) => ({ ...all, [p.id]: p }));
      if (p.error) setError(p.error);
      if (p.finished) load();
    });
    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  const download = (id: string) => {
    setError("");
    setProgress((all) => ({ ...all, [id]: { id, done: 0, total: 0, error: null, finished: false } }));
    invoke("offline_download", { id }).catch((e) => setError(String(e)));
  };

  const remove = async (id: string) => {
    await invoke("offline_remove", { id }).catch((e) => setError(String(e)));
    load();
  };

  return (
    <section>
      <h2>{t.offline_title}</h2>
      <p className="help">{t.offline_help}</p>
      <ul className="models">
        {models.map((model) => {
          const p = progress[model.id];
          const running = p && !p.finished && !p.error;
          return (
            <li key={model.id}>
              <span>
                {model.from === "ocr" ? t.model_ocr_label : `${langName(model.from, lang)} → ${langName(model.to, lang)}`}
                <small>{t.size_approx(model.size_mb)}</small>
              </span>
              {running ? (
                <span className="progress">
                  {p.total > 0 ? t.downloading(Math.round((p.done / p.total) * 100)) : t.test_busy}
                </span>
              ) : model.installed ? (
                <button onClick={() => remove(model.id)}>{t.delete_btn}</button>
              ) : (
                <button className="primary" onClick={() => download(model.id)}>
                  {t.download_btn}
                </button>
              )}
            </li>
          );
        })}
      </ul>
      {error && <p className="error">{error}</p>}
    </section>
  );
}
