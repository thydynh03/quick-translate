import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Box = { x0: number; y0: number; x1: number; y1: number };

/** A translated paragraph, in physical pixels relative to the overlay. */
type Block = { x: number; y: number; width: number; height: number; line_height: number; text: string; subtitle?: boolean };

/**
 * Full-screen overlay with two jobs: drag a rectangle to translate the text
 * inside it, or show the translations of a whole window over their source text.
 */
export default function Capture() {
  const [box, setBox] = useState<Box | null>(null);
  const [blocks, setBlocks] = useState<Block[] | null>(null);
  const [expected, setExpected] = useState(0);
  const [done, setDone] = useState(false);
  // Live mode: the shortcut that switches it off, while it is on.
  const [live, setLive] = useState<string | null>(null);
  // The part of the screen live mode follows, when it is not all of it.
  const [region, setRegion] = useState<number[] | null>(null);
  // Asking what live mode should follow, then dragging the region for it.
  const [choosing, setChoosing] = useState(false);
  const [forLive, setForLive] = useState(false);
  const isLive = useRef(false);
  // Where the overlay element really starts inside the window. Everything
  // drawn in it is placed relative to that, so screen pixels line up.
  const root = useRef<HTMLDivElement>(null);
  const [origin, setOrigin] = useState({ x: 0, y: 0 });
  useLayoutEffect(() => {
    const rect = root.current?.getBoundingClientRect();
    if (rect && (rect.left !== origin.x || rect.top !== origin.y)) setOrigin({ x: rect.left, y: rect.top });
  });

  const choosingNow = useRef(false);
  choosingNow.current = choosing;
  const pickRegion = () => {
    setChoosing(false);
    setForLive(true);
  };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") invoke("capture_cancel");
      if (!choosingNow.current) return;
      if (event.key === "1") invoke("live_begin", { region: null });
      if (event.key === "2") pickRegion();
      if (event.key === "3") invoke("live_listen");
    };
    window.addEventListener("keydown", onKey);
    const listeners = [
      listen("capture-start", () => {
        isLive.current = false;
        setLive(null);
        setBlocks(null);
        setBox(null);
        setChoosing(false);
        setForLive(false);
      }),
      listen("live-choose", () => {
        isLive.current = false;
        setLive(null);
        setBlocks(null);
        setBox(null);
        setForLive(false);
        setChoosing(true);
      }),
      listen<{ hotkey: string; region: number[] | null }>("live-start", (event) => {
        isLive.current = true;
        setLive(event.payload.hotkey);
        setRegion(event.payload.region);
        setChoosing(false);
        setForLive(false);
        setBlocks([]);
      }),
      // A reading that arrives after live mode ended must not bring it back.
      listen<Block[]>("live-frame", (event) => isLive.current && setBlocks(event.payload)),
      listen<number>("window-start", (event) => {
        isLive.current = false;
        setLive(null);
        setBlocks([]);
        setExpected(event.payload);
        setDone(event.payload === 0);
      }),
      listen<Block>("window-block", (event) => setBlocks((all) => (all ? [...all, event.payload] : all))),
      listen("window-done", () => setDone(true)),
    ];
    return () => {
      window.removeEventListener("keydown", onKey);
      listeners.forEach((unlisten) => unlisten.then((off) => off()));
    };
  }, []);

  if (blocks) {
    const scale = window.devicePixelRatio;
    const status = live !== null
      ? `Đang dịch thời gian thực · nhấn ${live} để tắt`
      : !done
      ? `Đang dịch ${blocks.length}/${expected} đoạn…`
      : blocks.length === 0
        ? "Không tìm thấy chữ cần dịch trong cửa sổ này"
        : "Bấm chuột hoặc nhấn Esc để đóng";
    return (
      <div ref={root} className={live !== null ? "capture result live" : "capture result"} onClick={() => invoke("capture_cancel")} onContextMenu={(e) => e.preventDefault()}>
        {live !== null && region && (
          <div
            className="capture-region"
            style={{
              left: region[0] / scale - origin.x,
              top: region[1] / scale - origin.y,
              width: region[2] / scale,
              height: region[3] / scale,
            }}
          />
        )}
        {blocks.map((block, index) =>
          block.subtitle ? (
            // Speech: a film subtitle, centred in the span it was given.
            <div key={index} className="capture-subtitle" style={{ left: block.x / scale - origin.x, top: block.y / scale - origin.y, width: block.width / scale }}>
              <p key={block.text}>{block.text}</p>
            </div>
          ) : (
          <p
            key={index}
            className="capture-text"
            style={{
              left: block.x / scale - origin.x,
              top: block.y / scale - origin.y,
              minWidth: block.width / scale,
              minHeight: block.height / scale,
              maxWidth: Math.max(block.width / scale, 260),
              // Text about as large as the lines it covers.
              fontSize: Math.min(Math.max((block.line_height / scale) * 0.72, 11), 22),
            }}
          >
            {block.text}
          </p>
          ),
        )}
        <p className="capture-hint" key={status}>
          {status}
        </p>
      </div>
    );
  }

  if (choosing) {
    return (
      <div ref={root} className="capture choose" onContextMenu={(e) => { e.preventDefault(); invoke("capture_cancel"); }}>
        <div className="capture-choice">
          <strong>Dịch thời gian thực</strong>
          <button autoFocus onClick={() => invoke("live_begin", { region: null })}>
            <kbd>1</kbd> Toàn màn hình
          </button>
          <button onClick={pickRegion}>
            <kbd>2</kbd> Chọn một vùng
          </button>
          <button onClick={() => invoke("live_listen")}>
            <kbd>3</kbd> Nghe tiếng (Live Captions)
          </button>
          <small>Esc để huỷ</small>
        </div>
      </div>
    );
  }

  const finish = (x1: number, y1: number) => {
    if (!box) return;
    box.x1 = x1;
    box.y1 = y1;
    const scale = window.devicePixelRatio;
    const left = Math.min(box.x0, box.x1);
    const top = Math.min(box.y0, box.y1);
    const area = {
      x: Math.round(left * scale),
      y: Math.round(top * scale),
      width: Math.round(Math.abs(box.x1 - box.x0) * scale),
      height: Math.round(Math.abs(box.y1 - box.y0) * scale),
    };
    if (forLive) invoke("live_begin", { region: [area.x, area.y, area.width, area.height] });
    else invoke("capture_region", area);
    setBox(null);
  };

  return (
    <div
      ref={root}
      className="capture"
      // Pointer capture keeps the drag alive even if the window is not focused.
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        setBox({ x0: e.clientX, y0: e.clientY, x1: e.clientX, y1: e.clientY });
      }}
      onPointerMove={(e) => box && setBox({ ...box, x1: e.clientX, y1: e.clientY })}
      onPointerUp={(e) => finish(e.clientX, e.clientY)}
      onContextMenu={(e) => {
        e.preventDefault();
        invoke("capture_cancel");
      }}
    >
      {box ? (
        <div
          className="capture-box"
          style={{
            left: Math.min(box.x0, box.x1) - origin.x,
            top: Math.min(box.y0, box.y1) - origin.y,
            width: Math.abs(box.x1 - box.x0),
            height: Math.abs(box.y1 - box.y0),
          }}
        />
      ) : (
        <p className="capture-hint">{forLive ? "Kéo để chọn vùng dịch thời gian thực · Esc để huỷ" : "Kéo để chọn vùng cần dịch · Esc để huỷ"}</p>
      )}
    </div>
  );
}
