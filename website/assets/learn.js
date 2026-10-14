// Flashcards for words saved in Swipelate. Cards arrive in the link (#d=…) from
// the desktop app (src/deck.ts packs them) and live in this browser's localStorage.
(() => {
  const KEY = "qt-deck-v1";
  const NEW_PER_DAY = 20;
  const DAY = 86400000;
  const $ = (id) => document.getElementById(id);

  // ------------------------------------------------------------ storage
  const blank = () => ({ cards: {}, removed: [], days: [], reverse: false, fresh: { day: "", count: 0 } });
  let db = blank();
  try {
    db = { ...blank(), ...JSON.parse(localStorage.getItem(KEY) || "{}") };
  } catch {}
  const save = () => {
    try {
      localStorage.setItem(KEY, JSON.stringify(db));
    } catch {
      toast("Không lưu được tiến độ (trình duyệt chặn bộ nhớ).");
    }
  };
  const keyOf = (c) => `${c.to}|${c.front.trim().toLowerCase()}`;
  const today = () => new Date().toLocaleDateString("sv");

  /** Adds new cards and updates the text of known ones; review progress is kept. */
  function merge(rows) {
    let added = 0;
    for (const [front, back, detail = "", from = "", to = ""] of rows) {
      if (!front) continue;
      const card = { front, back, detail, from, to };
      const key = keyOf(card);
      if (db.removed.includes(key)) continue; // deleted on the phone; scanning again keeps it deleted
      const old = db.cards[key];
      if (!old) added++;
      db.cards[key] = { ...card, added: old?.added ?? Date.now(), srs: old?.srs ?? null };
    }
    save();
    return added;
  }

  async function unpack(text) {
    const b64 = text.replace(/-/g, "+").replace(/_/g, "/");
    const bytes = Uint8Array.from(atob(b64 + "===".slice((b64.length + 3) % 4)), (ch) => ch.charCodeAt(0));
    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
    const [version, rows] = JSON.parse(await new Response(stream).text());
    if (version !== 1 || !Array.isArray(rows)) throw new Error("format");
    return rows;
  }

  // ---------------------------------------------------- spaced repetition
  /** Simplified SM-2. Returns the new state for grade 0 (forgot) … 3 (easy). */
  function next(srs, grade, now = Date.now()) {
    const s = srs ?? { reps: 0, ivl: 0, ease: 2.5 };
    let { reps, ivl, ease } = s;
    if (grade === 0) return { reps: 0, ivl: 0, ease: Math.max(1.3, ease - 0.2), due: now + 60000 };
    if (grade === 1) {
      ivl = Math.max(1, ivl * 1.2);
      ease = Math.max(1.3, ease - 0.15);
    } else if (grade === 2) {
      ivl = reps === 0 ? 1 : reps === 1 ? 3 : ivl * ease;
    } else {
      ivl = reps === 0 ? 4 : Math.max(ivl * ease * 1.3, 4);
      ease += 0.15;
    }
    ivl = Math.round(ivl * 10) / 10;
    return { reps: reps + 1, ivl, ease, due: now + ivl * DAY };
  }

  const label = (srs) => (srs.ivl === 0 ? "<1 phút" : srs.ivl < 30 ? `${Math.round(srs.ivl)} ngày` : `${Math.round(srs.ivl / 30)} tháng`);

  const all = () => Object.values(db.cards);
  const isDue = (c) => c.srs && c.srs.due <= Date.now();
  function newLeft() {
    if (db.fresh.day !== today()) db.fresh = { day: today(), count: 0 };
    return Math.max(0, NEW_PER_DAY - db.fresh.count);
  }

  // ----------------------------------------------------------------- views
  const views = ["home", "study", "quiz", "done"];
  const show = (id) => views.forEach((v) => ($(v).hidden = v !== id));

  function toast(text) {
    const t = $("toast");
    t.textContent = text;
    t.hidden = false;
  }

  function streak() {
    const days = new Set(db.days);
    let n = 0;
    for (let d = new Date(); days.has(d.toLocaleDateString("sv")); d = new Date(d - DAY)) n++;
    return n;
  }

  function home() {
    const cards = all();
    $("s-due").textContent = cards.filter(isDue).length;
    $("s-new").textContent = Math.min(cards.filter((c) => !c.srs).length, newLeft());
    $("s-total").textContent = cards.length;
    $("empty").hidden = cards.length > 0;
    $("start-study").disabled = $("start-quiz").disabled = !cards.length;
    $("reverse").textContent = db.reverse ? "Mặt trước: bản dịch" : "Mặt trước: từ gốc";
    const s = streak();
    $("streak").textContent = s ? `🔥 ${s} ngày` : "";
    renderList();
    show("home");
  }

  function renderList() {
    const needle = $("search").value.trim().toLowerCase();
    const list = $("list");
    list.replaceChildren();
    const cards = all()
      .filter((c) => !needle || `${c.front}\n${c.back}`.toLowerCase().includes(needle))
      .sort((a, b) => b.added - a.added);
    for (const card of cards.slice(0, 300)) {
      const li = document.createElement("li");
      const text = document.createElement("div");
      const front = document.createElement("p");
      front.textContent = card.front;
      const back = document.createElement("small");
      back.textContent = card.back + (card.srs ? ` · ôn lại sau ${label(card.srs)}` : " · chưa học");
      text.append(front, back);
      const del = document.createElement("button");
      del.textContent = "×";
      del.setAttribute("aria-label", `Xoá ${card.front}`);
      del.onclick = () => {
        if (!confirm(`Xoá thẻ "${card.front}"?`)) return;
        delete db.cards[keyOf(card)];
        db.removed = [...db.removed.slice(-2000), keyOf(card)];
        save();
        home();
      };
      li.append(text, del);
      list.append(li);
    }
  }

  // ------------------------------------------------------------ flashcards
  let queue = [];
  let current = null;

  function startStudy() {
    const due = all().filter(isDue).sort((a, b) => a.srs.due - b.srs.due);
    const fresh = all()
      .filter((c) => !c.srs)
      .sort((a, b) => a.added - b.added)
      .slice(0, newLeft());
    queue = [...due, ...fresh];
    if (!queue.length) return finish("Hôm nay đã ôn hết. Quay lại vào ngày mai nhé!");
    show("study");
    nextCard();
  }

  function sides(card) {
    const back = document.createElement("span");
    back.textContent = card.back;
    if (card.detail) {
      const d = document.createElement("span");
      d.className = "detail";
      d.textContent = card.detail;
      back.append(d);
    }
    const front = document.createElement("span");
    front.textContent = card.front;
    return db.reverse ? [back, front] : [front, back];
  }

  function nextCard() {
    current = queue.shift();
    if (!current) return finish("Xong lượt học. Giỏi lắm!");
    const [front, back] = sides(current);
    $("face-front").replaceChildren(front);
    $("face-back").replaceChildren(back);
    $("face-back").hidden = true;
    $("grades").hidden = true;
    $("hint").hidden = false;
    $("study-left").textContent = `Còn ${queue.length + 1} thẻ`;
    for (let g = 0; g < 4; g++) $(`g${g}`).textContent = label(next(current.srs, g));
  }

  function flip() {
    if (!current || !$("face-back").hidden) return;
    $("face-back").hidden = false;
    $("grades").hidden = false;
    $("hint").hidden = true;
  }

  function grade(g) {
    if (!current) return;
    if (!current.srs) db.fresh.count++;
    current.srs = next(current.srs, g);
    if (g === 0) queue.splice(Math.min(queue.length, 3), 0, current);
    markDay();
    save();
    nextCard();
  }

  function markDay() {
    const d = today();
    if (!db.days.includes(d)) db.days = [...db.days.slice(-400), d];
  }

  const VOICES = { en: "en-US", vi: "vi-VN", ja: "ja-JP", ko: "ko-KR", zh: "zh-CN", fr: "fr-FR", de: "de-DE", es: "es-ES" };
  function speak() {
    if (!current || !("speechSynthesis" in window)) return;
    speechSynthesis.cancel();
    const u = new SpeechSynthesisUtterance(current.front);
    u.lang = VOICES[current.from] || current.from || "en-US";
    speechSynthesis.speak(u);
  }

  // ------------------------------------------------------------------ quiz
  let score = { right: 0, total: 0 };
  let quizCards = [];

  function startQuiz() {
    const cards = all();
    if (cards.length < 2) return toast("Cần ít nhất 2 thẻ để làm trắc nghiệm.");
    quizCards = cards.sort(() => Math.random() - 0.5).slice(0, 10);
    score = { right: 0, total: 0 };
    show("quiz");
    askNext();
  }

  function askNext() {
    const card = quizCards.shift();
    if (!card) return finish(`Đúng ${score.right}/${score.total} câu.`);
    current = card;
    const [q, a] = db.reverse ? ["back", "front"] : ["front", "back"];
    $("quiz-q").textContent = card[q];
    $("quiz-score").textContent = `${score.right}/${score.total}`;
    const others = all()
      .filter((c) => c !== card && c[a] !== card[a] && c[q] !== card[q])
      .sort(() => Math.random() - 0.5)
      .slice(0, 3);
    const options = [card, ...others].sort(() => Math.random() - 0.5);
    const box = $("choices");
    box.replaceChildren();
    for (const option of options) {
      const b = document.createElement("button");
      b.textContent = option[a];
      b.onclick = () => {
        if ($("quiz-next").hidden === false) return;
        score.total++;
        if (option === card) score.right++;
        b.classList.add(option === card ? "right" : "wrong");
        [...box.children][options.indexOf(card)].classList.add("right");
        $("quiz-score").textContent = `${score.right}/${score.total}`;
        $("quiz-next").hidden = false;
        markDay();
        save();
      };
      box.append(b);
    }
    $("quiz-next").hidden = true;
  }

  function finish(text) {
    current = null;
    $("done-text").textContent = text;
    show("done");
  }

  // ---------------------------------------------------------------- backup
  function exportFile() {
    const blob = new Blob([JSON.stringify(db)], { type: "application/json" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = `swipelate-flashcards-${today()}.json`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 1000);
  }

  async function importFile(file) {
    try {
      const data = JSON.parse(await file.text());
      const list = (value) => (Array.isArray(value) ? value.filter((item) => typeof item === "string") : []);
      if (!data?.cards || typeof data.cards !== "object" || Array.isArray(data.cards)) throw new Error("format");
      // Build the merged copy first so a bad file changes nothing.
      const next = { ...db, cards: { ...db.cards } };
      const before = Object.keys(db.cards).length;
      for (const [key, card] of Object.entries(data.cards || {})) {
        const valid = typeof card?.front === "string" && card.front.trim() && typeof card.back === "string";
        if (valid && !next.cards[key]) next.cards[key] = card;
      }
      next.days = [...new Set([...db.days, ...list(data.days)])].sort().slice(-400);
      next.removed = [...new Set([...db.removed, ...list(data.removed)])].slice(-2000);
      if (data.fresh?.day === today() && data.fresh.count > (db.fresh.day === today() ? db.fresh.count : 0)) next.fresh = data.fresh;
      db = next;
      save();
      home();
      toast(`Đã nhập ${Object.keys(db.cards).length - before} thẻ mới.`);
    } catch {
      toast("Tệp không đúng định dạng.");
    }
  }

  // ---------------------------------------------------------------- wiring
  $("start-study").onclick = startStudy;
  $("start-quiz").onclick = startQuiz;
  $("flash").onclick = flip;
  $("speak").onclick = speak;
  $("grades").onclick = (e) => {
    const b = e.target.closest("[data-grade]");
    if (b) grade(Number(b.dataset.grade));
  };
  $("quiz-next").onclick = askNext;
  $("search").oninput = renderList;
  $("export").onclick = exportFile;
  $("import").onchange = (e) => e.target.files[0] && importFile(e.target.files[0]);
  $("reverse").onclick = () => {
    db.reverse = !db.reverse;
    save();
    home();
  };
  document.querySelectorAll("[data-home]").forEach((b) => (b.onclick = home));
  document.addEventListener("keydown", (e) => {
    if ($("study").hidden) return;
    if ((e.key === " " || e.key === "Enter") && !e.target.closest?.("button:not(#flash)")) {
      e.preventDefault();
      flip();
    } else if (!$("grades").hidden && "1234".includes(e.key)) grade(Number(e.key) - 1);
  });

  if ("serviceWorker" in navigator) navigator.serviceWorker.register("learn-sw.js").catch(() => {});

  async function load() {
    const match = location.hash.match(/^#d=([\w-]+)/);
    if (!match) return home();
    history.replaceState(null, "", location.pathname);
    try {
      const added = merge(await unpack(match[1]));
      home();
      toast(added ? `Đã thêm ${added} thẻ mới.` : "Bộ thẻ đã có sẵn, không có thẻ mới.");
    } catch {
      home();
      toast("Liên kết bộ thẻ bị hỏng. Quét lại mã QR trên máy tính.");
    }
  }
  addEventListener("hashchange", load);
  load();
})();
