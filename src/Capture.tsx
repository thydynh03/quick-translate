import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Box = { x0: number; y0: number; x1: number; y1: number };

/** Full-screen overlay: drag a rectangle to translate the text inside it. */
export default function Capture() {
  const [box, setBox] = useState<Box | null>(null);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") invoke("capture_cancel");
    };
    window.addEventListener("keydown", onKey);
    const unlisten = listen("capture-start", () => setBox(null));
    return () => {
      window.removeEventListener("keydown", onKey);
      unlisten.then((off) => off());
    };
  }, []);

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
            left: Math.min(box.x0, box.x1),
            top: Math.min(box.y0, box.y1),
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
