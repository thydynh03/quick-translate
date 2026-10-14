// Packs saved cards into a link the phone opens: website/learn.html#d=<deflate, base64url>.
// The format is shared with website/assets/learn.js; change both together.
import qrcode from "qrcode-generator";
import type { Card } from "./types";

export const LEARN_URL = "https://swipelate.com/learn.html";
/** Longest link that still fits a QR code a phone camera reads from a screen. */
const QR_LIMIT = 1600;

async function pack(cards: Card[]): Promise<string> {
  if (typeof CompressionStream === "undefined") throw new Error("hệ điều hành quá cũ để tạo liên kết (cần macOS 13.3+ hoặc Windows 10+)");
  const rows = cards.map((c) => [c.front, c.back, c.detail, c.from, c.to]);
  const stream = new Blob([JSON.stringify([1, rows])]).stream().pipeThrough(new CompressionStream("deflate-raw"));
  const bytes = new Uint8Array(await new Response(stream).arrayBuffer());
  let binary = "";
  bytes.forEach((b) => (binary += String.fromCharCode(b)));
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

export async function deckLink(cards: Card[]): Promise<string> {
  return `${LEARN_URL}#d=${await pack(cards)}`;
}

/** The newest cards whose link fits in one QR code, found by halving. */
export async function qrDeck(cards: Card[]): Promise<{ url: string; count: number; svg: string }> {
  let low = 0;
  let high = cards.length;
  let best = await deckLink([]);
  while (low < high) {
    const mid = Math.ceil((low + high) / 2);
    const url = await deckLink(cards.slice(0, mid));
    if (url.length <= QR_LIMIT) {
      low = mid;
      best = url;
    } else {
      high = mid - 1;
    }
  }
  const qr = qrcode(0, "L");
  qr.addData(best);
  qr.make();
  return { url: best, count: low, svg: qr.createSvgTag({ cellSize: 3, margin: 4, scalable: true }) };
}
