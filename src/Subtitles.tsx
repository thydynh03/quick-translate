import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { openUrl } from "@tauri-apps/plugin-opener";

// Where the YouTube extension is offered. Point this at the store page once it is listed there.
const EXTENSION_URL = "https://thydynh03.github.io/quick-translate/docs.html#youtube";

type Job = { path: string; done: number; total: number; output?: string; error?: string };

const name = (path: string) => path.split(/[\\/]/).pop() ?? path;

/** Translates dropped subtitle files ahead of time, so a player shows them with no delay. */
export default function Subtitles() {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [over, setOver] = useState(false);
  // Files are translated one after the other, so an online engine is not flooded.
  const queue = useRef(Promise.resolve());

  const patch = (path: string, change: Partial<Job>) =>
    setJobs((all) => all.map((job) => (job.path === path ? { ...job, ...change } : job)));

  useEffect(() => {
    const add = (paths: string[]) => {
      const files = paths.filter((path) => /\.(srt|vtt)$/i.test(path));
      if (files.length === 0) return;
      setJobs((all) => [...all.filter((job) => !files.includes(job.path)), ...files.map((path) => ({ path, done: 0, total: 0 }))]);
      for (const path of files) {
        queue.current = queue.current.then(() =>
          invoke<string>("subtitle_translate", { path }).then(
            (output) => patch(path, { output }),
            (error) => patch(path, { error: String(error) }),
          ),
        );
      }
    };
    const dropped = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over" || event.payload.type === "enter") setOver(true);
      else setOver(false);
      if (event.payload.type === "drop") add(event.payload.paths);
    });
    const progressed = listen<Job>("subtitle-progress", (event) => patch(event.payload.path, event.payload));
    return () => {
      dropped.then((off) => off());
      progressed.then((off) => off());
    };
  }, []);

  return (
    <section>
      <h2>Phụ đề video</h2>
      <p className="hint">
        Dịch trước cả file phụ đề (.srt hoặc .vtt), nên khi xem không có độ trễ nào. File dịch được ghi cạnh file gốc với tên
        kiểu <code>phim.vi.srt</code>; trình phát như VLC hay MPV sẽ tự nhận khi tên bắt đầu giống tên file video.
      </p>
      <div className={over ? "dropzone over" : "dropzone"}>Kéo file .srt hoặc .vtt vào đây</div>
      {jobs.map((job) => (
        <div className="subtitle-job" key={job.path}>
          <strong>{name(job.path)}</strong>
          {job.error ? (
            <span className="error">{job.error}</span>
          ) : job.output ? (
            <span>Xong: {job.output}</span>
          ) : job.total > 0 ? (
            <span>
              Đang dịch {job.done}/{job.total} dòng…
            </span>
          ) : (
            <span>Đang chờ…</span>
          )}
          {!job.output && !job.error && <progress max={job.total || 1} value={job.done} />}
        </div>
      ))}
      <h2>Phụ đề YouTube</h2>
      <p className="hint">
        Video YouTube cần một extension cho Chrome hoặc Edge: nó lấy phụ đề tiếng Việt của cả video một lần rồi hiện đúng
        thời điểm, nên không có độ trễ. Extension chạy độc lập, không cần mở ứng dụng này.
      </p>
      <button onClick={() => openUrl(EXTENSION_URL)}>Cài extension cho YouTube</button>
    </section>
  );
}
