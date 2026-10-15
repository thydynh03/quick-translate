import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { deckLink, qrDeck } from "./deck";
import type { Card } from "./types";
import { AppLanguage, useT } from "./i18n";

/** Saved words: search, remove, export, import, and send to the phone as flashcards. */
export default function Vocab({ lang = "vi" }: { lang?: AppLanguage }) {
  const t = useT(lang);
  const isEn = lang === "en";
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
  const attempt = (action: () => Promise<unknown>) => () => action().catch((error) => setNote(isEn ? `Failed: ${error}` : `Không thực hiện được: ${error}`));

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
    setNote(isEn ? "Link copied. Send to your phone (email, chat…) and open." : "Đã sao chép liên kết. Gửi sang điện thoại (Zalo, email…) rồi mở.");
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
    setNote(isEn ? `Exported ${cards.length} cards to CSV (compatible with Anki and Excel).` : `Đã xuất ${cards.length} thẻ ra tệp CSV (dùng được cho Anki và Excel).`);
  });

  const exportJson = attempt(async () => {
    const json = JSON.stringify(cards, null, 2);
    const blob = new Blob([json], { type: "application/json;charset=utf-8;" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `swipelate-backup-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
    setNote(isEn ? `Backed up ${cards.length} cards to JSON file.` : `Đã sao lưu ${cards.length} thẻ ra tệp JSON.`);
  });

  const handleImportJson = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = async (event) => {
      try {
        const text = event.target?.result as string;
        const importedCards: Card[] = JSON.parse(text);
        if (!Array.isArray(importedCards)) {
          setNote(isEn ? "Invalid file format: JSON array expected." : "Tệp không đúng định dạng: cần mảng JSON các thẻ.");
          return;
        }
        const added = await invoke<number>("vocab_import_cards", { cards: importedCards });
        await load();
        setNote(isEn ? `Imported ${added} new cards (${importedCards.length - added} duplicates skipped).` : `Đã nhập thành công ${added} thẻ mới (bỏ qua ${importedCards.length - added} thẻ trùng lặp).`);
      } catch (err) {
        setNote(isEn ? `Failed to read file: ${err}` : `Không đọc được tệp: ${err}`);
      }
    };
    reader.readAsText(file);
    e.target.value = "";
  };

  const clearMatches = attempt(async () => {
    const count = await invoke<number>("vocab_clear_matches", { query: needle });
    setQuery("");
    setNote(isEn ? `Deleted ${count} search results.` : `Đã xoá ${count} thẻ theo kết quả tìm kiếm.`);
    await load();
  });

  return (
    <section>
      <h2>{t.vocab_title}</h2>
      {note && <p className="note">{note}</p>}
      <input
        type="file"
        ref={fileInputRef}
        style={{ display: "none" }}
        accept="application/json,.json"
        onChange={handleImportJson}
      />
      {cards.length === 0 ? (
        <>
          <p className="help">{t.vocab_empty}</p>
          <p className="row-buttons">
            <button onClick={() => fileInputRef.current?.click()}>
              {t.import_json}
            </button>
          </p>
        </>
      ) : (
        <>
          <input
            className="search"
            type="search"
            placeholder={t.vocab_search_ph(cards.length)}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setVisibleCount(40);
              setQr(null);
            }}
          />
          <p className="row-buttons">
            <button onClick={showQr} disabled={!shown.length}>
              {t.learn_phone}
            </button>
            <button onClick={copyLink} disabled={!shown.length}>
              {t.copy_link}
            </button>
            <button onClick={openHere} disabled={!shown.length}>
              {isEn ? "Open on this PC" : "Mở trên máy này"}
            </button>
            <button onClick={exportCsv} disabled={!cards.length} title="Export to CSV compatible with Anki and Excel">
              {t.export_csv}
            </button>
            <button onClick={exportJson} disabled={!cards.length} title="Backup all vocabulary cards to JSON">
              {t.backup_json}
            </button>
            <button onClick={() => fileInputRef.current?.click()} title="Import vocabulary cards from JSON file">
              {t.import_json}
            </button>
            {needle && (
              <button className="danger" onClick={clearMatches}>
                {t.clear_vocab_matches(shown.length)}
              </button>
            )}
          </p>
          {qr && qr.count === 0 && <p className="help">{isEn ? "Newest card is too long for QR code. Use 'Copy link'." : "Thẻ mới nhất quá dài để vừa một mã QR. Dùng 'Sao chép liên kết'."}</p>}
          {qr && qr.count > 0 && (
            <div className="qr">
              <div dangerouslySetInnerHTML={{ __html: qr.svg }} />
              <p className="help">
                {qr.count < shown.length
                  ? (isEn ? `QR code holds newest ${qr.count} cards. Use 'Copy link' to transfer all ${shown.length} cards.` : `Mã QR chứa ${qr.count} thẻ mới nhất (giới hạn dung lượng QR). Dùng 'Sao chép liên kết' để gửi đủ ${shown.length} thẻ.`)
                  : (isEn ? `QR code contains ${qr.count} cards. Scan again when saving more words; existing review history is preserved.` : `Mã QR chứa ${qr.count} thẻ. Quét lại khi lưu thêm từ, tiến độ học cũ được giữ.`)}
              </p>
            </div>
          )}
          {shown.length === 0 ? (
            <p className="help">{t.no_vocab_matches}</p>
          ) : (
            <>
              <p className="help count-hint">
                {t.showing_vocab(Math.min(visibleCount, shown.length), shown.length)}
              </p>
              <ul className="history">
                {shown.slice(0, visibleCount).map((card) => (
                  <li key={`${card.to}-${card.front}`}>
                    <small>
                      {new Date(card.at * 1000).toLocaleDateString(isEn ? "en-US" : "vi-VN")} · {card.from.toUpperCase()} → {card.to.toUpperCase()} ·{" "}
                      <button className="link" onClick={() => remove(card)}>
                        {isEn ? "Delete" : "Xoá"}
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
                    {t.load_more(40, shown.length - visibleCount)}
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
