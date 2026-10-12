import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { deckLink, qrDeck } from "./deck";
import type { Card } from "./types";

/** Saved words: search, remove, and send to the phone as flashcards. */
export default function Vocab() {
  const [cards, setCards] = useState<Card[]>([]);
  const [query, setQuery] = useState("");
  const [qr, setQr] = useState<{ count: number; svg: string } | null>(null);
  const [note, setNote] = useState("");

  const load = () => invoke<Card[]>("vocab_list").then(setCards);
  useEffect(() => {
    load();
  }, []);

  const needle = query.trim().toLowerCase();
  const shown = needle ? cards.filter((c) => `${c.front}\n${c.back}`.toLowerCase().includes(needle)) : cards;

  const remove = async (card: Card) => {
    await invoke("vocab_remove", { front: card.front, to: card.to });
    setQr(null);
    load();
  };

  const showQr = async () => {
    const deck = await qrDeck(shown);
    setQr(deck);
  };

  const copyLink = async () => {
    await invoke("copy_text", { text: await deckLink(shown) });
    setNote("Đã sao chép liên kết. Gửi sang điện thoại (Zalo, email…) rồi mở.");
  };

  return (
    <section>
      <h2>Từ vựng</h2>
      <p className="help">
        Bấm nút đánh dấu trên popup dịch để lưu từ hoặc câu. Học trên điện thoại bằng flashcard: quét mã QR, trang học tự nhớ tiến độ và
        nhắc ôn theo lịch lặp lại ngắt quãng. Thẻ chỉ nằm trên máy bạn và trong liên kết, không gửi lên máy chủ nào.
      </p>
      {cards.length === 0 ? (
        <p className="help">Chưa lưu từ nào.</p>
      ) : (
        <>
          <input className="search" type="search" placeholder={`Tìm trong ${cards.length} thẻ`} value={query} onChange={(e) => setQuery(e.target.value)} />
          <p className="row-buttons">
            <button onClick={showQr} disabled={!shown.length}>
              Học trên điện thoại (mã QR)
            </button>
            <button onClick={copyLink} disabled={!shown.length}>
              Sao chép liên kết
            </button>
            <button onClick={async () => openUrl(await deckLink(shown))} disabled={!shown.length}>
              Mở trên máy này
            </button>
          </p>
          {note && <p className="help">{note}</p>}
          {qr && (
            <div className="qr">
              <div dangerouslySetInnerHTML={{ __html: qr.svg }} />
              <p className="help">
                {qr.count < shown.length
                  ? `Mã QR chứa ${qr.count} thẻ mới nhất (giới hạn dung lượng QR). Dùng "Sao chép liên kết" để gửi đủ ${shown.length} thẻ.`
                  : `Mã QR chứa ${qr.count} thẻ. Quét lại khi lưu thêm từ, tiến độ học cũ được giữ.`}
              </p>
            </div>
          )}
          <ul className="history">
            {shown.map((card) => (
              <li key={`${card.to}-${card.front}`}>
                <small>
                  {new Date(card.at * 1000).toLocaleDateString("vi-VN")} · {card.from.toUpperCase()} → {card.to.toUpperCase()} ·{" "}
                  <button className="link" onClick={() => remove(card)}>
                    Xoá
                  </button>
                </small>
                <p className="from">{card.front}</p>
                <p>{card.back}</p>
                {card.detail && <p className="detail">{card.detail}</p>}
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}
