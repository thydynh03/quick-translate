import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { deckLink, qrDeck } from "./deck";
import type { Card } from "./types";

/** Saved words: search, remove, export, import, and send to the phone as flashcards. */
export default function Vocab() {
  const [cards, setCards] = useState<Card[]>([]);
  const [query, setQuery] = useState("");
  const [qr, setQr] = useState<{ count: number; svg: string } | null>(null);
  const [note, setNote] = useState("");
  const [visibleCount, setVisibleCount] = useState(40);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const load = () =>
    invoke<Card[]>("vocab_list")
      .then(setCards)
      .catch((error) => setNote(String(error)));

  useEffect(() => {
    load();
  }, []);

  const needle = query.trim().toLowerCase();
  const shown = needle ? cards.filter((c) => `${c.front}\n${c.back}`.toLowerCase().includes(needle)) : cards;

  // Runs an action and shows its error instead of failing silently.
  const attempt = (action: () => Promise<unknown>) => () => action().catch((error) => setNote(`Không thực hiện được: ${error}`));

  const remove = (card: Card) =>
    attempt(async () => {
      await invoke("vocab_remove", { front: card.front, to: card.to });
      setQr(null);
      await load();
    })();

  const showQr = attempt(async () => {
    setNote("");
    setQr(await qrDeck(shown));
  });

  const copyLink = attempt(async () => {
    await invoke("copy_text", { text: await deckLink(shown) });
    setNote("Đã sao chép liên kết. Gửi sang điện thoại (Zalo, email…) rồi mở.");
  });

  const openHere = attempt(async () => openUrl(await deckLink(shown)));

  const exportCsv = attempt(async () => {
    const csv = await invoke<string>("vocab_export_csv");
    const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `swipelate-vocab-${new Date().toISOString().slice(0, 10)}.csv`;
    a.click();
    URL.revokeObjectURL(url);
    setNote(`Đã xuất ${cards.length} thẻ ra tệp CSV (dùng được cho Anki và Excel).`);
  });

  const exportJson = attempt(async () => {
    const blob = new Blob([JSON.stringify(cards, null, 2)], { type: "application/json;charset=utf-8;" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `swipelate-vocab-backup-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
    setNote(`Đã sao lưu ${cards.length} thẻ ra tệp JSON.`);
  });

  const importJson = (file: File) => {
    const reader = new FileReader();
    reader.onload = async (e) => {
      try {
        const text = e.target?.result as string;
        const imported = JSON.parse(text);
        if (!Array.isArray(imported)) {
          throw new Error("Tệp không đúng định dạng (cần danh sách thẻ).");
        }
        const count = await invoke<number>("vocab_import_cards", { cards: imported });
        setNote(`Đã nhập thành công ${count} thẻ mới.`);
        await load();
      } catch (err) {
        setNote(`Không thể nhập thẻ: ${err}`);
      }
    };
    reader.readAsText(file);
  };

  const clearMatches = attempt(async () => {
    if (!needle) return;
    const count = await invoke<number>("vocab_clear_matches", { query: query.trim() });
    setNote(`Đã xoá ${count} thẻ phù hợp.`);
    setQr(null);
    await load();
  });

  return (
    <section>
      <h2>Từ vựng</h2>
      <p className="help">
        Bấm nút đánh dấu trên popup dịch để lưu từ hoặc câu. Học trên điện thoại bằng flashcard: quét mã QR, trang học tự nhớ tiến độ và
        nhắc ôn theo lịch lặp lại ngắt quãng. Thẻ chỉ nằm trên máy bạn và trong liên kết, không gửi lên máy chủ nào.
      </p>
      {note && <p className="help">{note}</p>}
      <input
        type="file"
        ref={fileInputRef}
        style={{ display: "none" }}
        accept=".json"
        onChange={(e) => {
          const file = e.target.files?.[0];
          if (file) {
            importJson(file);
            e.target.value = "";
          }
        }}
      />
      {cards.length === 0 ? (
        <>
          <p className="help">Chưa lưu từ nào.</p>
          <p className="row-buttons">
            <button onClick={() => fileInputRef.current?.click()}>
              Nhập thẻ từ tệp sao lưu JSON…
            </button>
          </p>
        </>
      ) : (
        <>
          <input
            className="search"
            type="search"
            placeholder={`Tìm trong ${cards.length} thẻ…`}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setVisibleCount(40);
              setQr(null);
            }}
          />
          <p className="row-buttons">
            <button onClick={showQr} disabled={!shown.length}>
              Học trên điện thoại (mã QR)
            </button>
            <button onClick={copyLink} disabled={!shown.length}>
              Sao chép liên kết
            </button>
            <button onClick={openHere} disabled={!shown.length}>
              Mở trên máy này
            </button>
            <button onClick={exportCsv} disabled={!cards.length} title="Xuất ra tệp CSV tương thích Anki và Excel">
              Xuất CSV (Anki)
            </button>
            <button onClick={exportJson} disabled={!cards.length} title="Sao lưu toàn bộ thẻ từ vựng ra tệp JSON">
              Sao lưu JSON
            </button>
            <button onClick={() => fileInputRef.current?.click()} title="Nhập thẻ từ vựng từ tệp JSON đã sao lưu">
              Nhập JSON…
            </button>
            {needle && (
              <button className="danger" onClick={clearMatches}>
                Xoá {shown.length} thẻ tìm thấy
              </button>
            )}
          </p>
          {qr && qr.count === 0 && <p className="help">Thẻ mới nhất quá dài để vừa một mã QR. Dùng "Sao chép liên kết".</p>}
          {qr && qr.count > 0 && (
            <div className="qr">
              <div dangerouslySetInnerHTML={{ __html: qr.svg }} />
              <p className="help">
                {qr.count < shown.length
                  ? `Mã QR chứa ${qr.count} thẻ mới nhất (giới hạn dung lượng QR). Dùng "Sao chép liên kết" để gửi đủ ${shown.length} thẻ.`
                  : `Mã QR chứa ${qr.count} thẻ. Quét lại khi lưu thêm từ, tiến độ học cũ được giữ.`}
              </p>
            </div>
          )}
          {shown.length === 0 ? (
            <p className="help">Không tìm thấy thẻ nào phù hợp.</p>
          ) : (
            <>
              <p className="help count-hint">
                Đang hiển thị {Math.min(visibleCount, shown.length)} trên {shown.length} thẻ
              </p>
              <ul className="history">
                {shown.slice(0, visibleCount).map((card) => (
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
              {shown.length > visibleCount && (
                <div className="pagination">
                  <button onClick={() => setVisibleCount((c) => c + 40)}>
                    Tải thêm 40 thẻ ({shown.length - visibleCount} thẻ còn lại)
                  </button>
                </div>
              )}
            </>
          )}
        </>
      )}
    </section>
  );
}
