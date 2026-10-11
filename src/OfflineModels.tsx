import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { langName } from "./types";

type Model = {
  id: string;
  from: string;
  to: string;
  size_mb: number;
  installed: boolean;
};

type Progress = { id: string; done: number; total: number; error: string | null; finished: boolean };

export default function OfflineModels() {
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
      <h2>Model offline</h2>
      <p className="help">Mỗi model dịch một chiều. Tải một lần, sau đó dịch không cần mạng.</p>
      <ul className="models">
        {models.map((model) => {
          const p = progress[model.id];
          const running = p && !p.finished && !p.error;
          return (
            <li key={model.id}>
              <span>
                {langName(model.from)} → {langName(model.to)}
                <small>khoảng {model.size_mb} MB</small>
              </span>
              {running ? (
                <span className="progress">
                  {p.total > 0 ? `${Math.round((p.done / p.total) * 100)}%` : "Đang tải…"}
                </span>
              ) : model.installed ? (
                <button onClick={() => remove(model.id)}>Xoá</button>
              ) : (
                <button className="primary" onClick={() => download(model.id)}>
                  Tải về
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
