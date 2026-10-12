import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Box = { x0: number; y0: number; x1: number; y1: number };

/** A translated paragraph, in physical pixels relative to the overlay. */
type Block = { x: number; y: number; width: number; height: number; line_height: number; text: string };

/**
 * Full-screen overlay with two jobs: drag a rectangle to translate the text
 * inside it, or show the translations of a whole window over their source text.
 */
export default function Capture() {
  const [box, setBox] = useState<Box | null>(null);
  const [blocks, setBlocks] = useState<Block[] | null>(null);
  const [expected, setExpected] = useState(0);
  const [done, setDone] = useState(false);
  // Where the overlay element really starts inside the window. Everything
  // drawn in it is placed relative to that, so screen pixels line up.
  const root = useRef<HTMLDivElement>(null);
  const [origin, setOrigin] = useState({ x: 0, y: 0 });
  useLayoutEffect(() => {
    const rect = root.current?.getBoundingClientRect();
    if (rect && (rect.left !== origin.x || rect.top !== origin.y)) setOrigin({ x: rect.left, y: rect.top });
  });

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") invoke("capture_cancel");
    };
    window.addEventListener("keydown", onKey);
    const listeners = [
      listen("capture-start", () => {
        setBlocks(null);
        setBox(null);
      }),
      listen<number>("window-start", (event) => {
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
    const status = !done
      ? `Đang dịch ${blocks.length}/${expected} đoạn…`
      : blocks.length === 0
        ? "Không tìm thấy chữ cần dịch trong cửa sổ này"
        : "Bấm chuột hoặc nhấn Esc để đóng";
    return (
      <div ref={root} className="capture result" onClick={() => invoke("capture_cancel")} onContextMenu={(e) => e.preventDefault()}>
        {blocks.map((block, index) => (
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
        ))}
        <p className="capture-hint">{status}</p>
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
    invoke("capture_region", {
      x: Math.round(left * scale),
      y: Math.round(top * scale),
      width: Math.round(Math.abs(box.x1 - box.x0) * scale),
      height: Math.round(Math.abs(box.y1 - box.y0) * scale),
    });
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
        <p className="capture-hint">Kéo để chọn vùng cần dịch · Esc để huỷ</p>
      )}
    </div>
  );
}
