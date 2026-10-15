// Dashboard: downloads from the GitHub Releases API, users from the stats server (server/).
// Every number on the page comes from one of those two sources; nothing is filled in by hand.
(() => {
  const REPO = "thydynh03/swipelate";
  const STATS = (window.QT_STATS_URL || "").replace(/\/$/, "");
  const NS = "http://www.w3.org/2000/svg";
  const $ = (id) => document.getElementById(id);
  const fmt = (n) => Number(n || 0).toLocaleString("vi-VN");
  const pct = (part, whole) => (whole ? `${Math.round((part / whole) * 100)}%` : "0%");
  const dateVi = (iso) => new Date(iso).toLocaleDateString("vi-VN", { day: "2-digit", month: "2-digit", year: "numeric" });

  const isWindows = (name) => /\.(exe|msi)$/i.test(name);
  const isMac = (name) => /\.dmg$/i.test(name);

  const redraws = {};

  // ------------------------------------------------------------- helpers
  function svg(tag, attrs = {}, parent) {
    const el = document.createElementNS(NS, tag);
    for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
    if (parent) parent.append(el);
    return el;
  }

  function setText(scope, key, text) {
    const el = scope.querySelector(`[data-k="${key}"]`);
    if (el) el.textContent = text;
  }

  /** A "nice" axis maximum and step for `max`, giving about 4 gridlines. */
  function niceScale(max) {
    if (max <= 4) return { top: 4, step: 1 };
    const raw = max / 4;
    const mag = 10 ** Math.floor(Math.log10(raw));
    const step = [1, 2, 2.5, 5, 10].map((m) => m * mag).find((s) => s >= raw);
    return { top: Math.ceil(max / step) * step, step };
  }

  /** Path for a bar with 4px rounded top corners, anchored square to the baseline. */
  function barPath(x, y, w, h, r = 4) {
    if (h <= 0) return "";
    r = Math.min(r, w / 2, h);
    return `M${x},${y + h}V${y + r}Q${x},${y} ${x + r},${y}H${x + w - r}Q${x + w},${y} ${x + w},${y + r}V${y + h}Z`;
  }

  const tip = $("tip");
  function showTip(html, event) {
    tip.innerHTML = html;
    tip.hidden = false;
    const pad = 14;
    const { innerWidth: W, innerHeight: H } = window;
    const r = tip.getBoundingClientRect();
    let x = event.clientX + pad;
    let y = event.clientY + pad;
    if (x + r.width > W - 8) x = event.clientX - r.width - pad;
    if (y + r.height > H - 8) y = event.clientY - r.height - pad;
    tip.style.left = `${x}px`;
    tip.style.top = `${y}px`;
  }
  const hideTip = () => (tip.hidden = true);
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

  function table(container, head, rows) {
    const t = document.createElement("table");
    const tr = t.createTHead().insertRow();
    head.forEach(([label, num]) => {
      const th = document.createElement("th");
      th.textContent = label;
      if (num) th.className = "n";
      tr.append(th);
    });
    const body = t.createTBody();
    rows.forEach((cells) => {
      const r = body.insertRow();
      cells.forEach((value, i) => {
        const td = r.insertCell();
        td.textContent = value;
        if (head[i][1]) td.className = "n";
      });
    });
    container.replaceChildren(t);
  }

  document.querySelectorAll("[data-table]").forEach((button) => {
    button.addEventListener("click", () => {
      const id = button.dataset.table;
      const tableEl = $(`${id}-table`);
      const chartEl = $(`${id}-chart`);
      const showTable = tableEl.hidden;
      tableEl.hidden = !showTable;
      chartEl.hidden = showTable;
      const isEn = window.SwipelateI18n?.getActiveLang?.() === "en";
      button.textContent = showTable
        ? (isEn ? "View as chart" : "Xem dạng biểu đồ")
        : (isEn ? "View as table" : "Xem dạng bảng");
    });
  });

  // ------------------------------------------------------ stacked bars
  function stackedBars(container, items) {
    const W = Math.max(320, container.clientWidth || 720);
    const dated = (W - 44) / items.length > 70;
    const H = dated ? 280 : 220;
    const m = { top: 22, right: 8, bottom: dated ? 44 : 28, left: 36 };
    const iw = W - m.left - m.right;
    const ih = H - m.top - m.bottom;
    const max = Math.max(0, ...items.map((d) => d.win + d.mac));
    const { top, step } = niceScale(max);
    const y = (v) => m.top + ih - (v / top) * ih;
    const slot = iw / items.length;
    const bw = Math.min(44, slot * 0.56);

    const root = svg("svg", { viewBox: `0 0 ${W} ${H}`, "aria-hidden": "true" });
    for (let v = 0; v <= top; v += step) {
      svg("line", { class: v ? "grid" : "axis", x1: m.left, x2: W - m.right, y1: y(v), y2: y(v) }, root);
      svg("text", { x: m.left - 8, y: y(v) + 4, "text-anchor": "end" }, root).textContent = fmt(v);
    }

    const hits = [];
    items.forEach((d, i) => {
      const cx = m.left + slot * i + slot / 2;
      const band = svg("rect", { class: "band", x: cx - slot / 2 + 2, y: m.top, width: slot - 4, height: ih, rx: 6, fill: "transparent" }, root);
      const x = cx - bw / 2;
      const hWin = (d.win / top) * ih;
      const hMac = (d.mac / top) * ih;
      const gap = d.win && d.mac ? 2 : 0;
      // Windows sits on the baseline; macOS stacks above it with a 2px surface gap.
      if (d.win) svg("path", { d: d.mac ? `M${x},${y(0)}V${y(0) - hWin}H${x + bw}V${y(0)}Z` : barPath(x, y(0) - hWin, bw, hWin), fill: "var(--series-1)" }, root);
      if (d.mac) svg("path", { d: barPath(x, y(0) - hWin - gap - hMac, bw, hMac), fill: "var(--series-2)" }, root);
      const total = d.win + d.mac;
      svg("text", { class: "val", x: cx, y: y(total) - (gap + 6), "text-anchor": "middle" }, root).textContent = fmt(total);
      const label = svg("text", { x: cx, y: H - m.bottom + 18, "text-anchor": "middle" }, root);
      label.textContent = d.tag;
      if (dated) svg("text", { x: cx, y: H - m.bottom + 33, "text-anchor": "middle", "font-size": 11 }, root).textContent = d.date;

      hits.push({ cx, band, html: null, total });
      const html = `<b>${esc(d.tag)}</b> · ${esc(d.date)}<div class="row"><i class="sw s1"></i>Windows <b>${fmt(d.win)}</b></div><div class="row"><i class="sw s2"></i>macOS <b>${fmt(d.mac)}</b></div><div>Tổng <b>${fmt(total)}</b></div>`;
      hits[hits.length - 1].html = html;
    });
    // Hit areas go last so labels drawn for later columns never sit on top of them.
    for (const { cx, band, html } of hits) {
      const hit = svg("rect", { class: "hit", x: cx - slot / 2, y: m.top, width: slot, height: ih + m.bottom }, root);
      hit.addEventListener("pointermove", (e) => {
        band.classList.add("on");
        showTip(html, e);
      });
      hit.addEventListener("pointerleave", () => {
        band.classList.remove("on");
        hideTip();
      });
    }
    container.replaceChildren(root);
  }

  // ------------------------------------------------------------ lines
  function lineChart(container, days, series) {
    const W = Math.max(320, container.clientWidth || 720);
    const H = W < 520 ? 220 : 260;
    const m = { top: 16, right: 12, bottom: 30, left: 36 };
    const iw = W - m.left - m.right;
    const ih = H - m.top - m.bottom;
    const max = Math.max(0, ...series.flatMap((s) => s.values));
    const { top, step } = niceScale(max);
    const x = (i) => m.left + (days.length < 2 ? iw / 2 : (i / (days.length - 1)) * iw);
    const y = (v) => m.top + ih - (v / top) * ih;

    const root = svg("svg", { viewBox: `0 0 ${W} ${H}`, "aria-hidden": "true" });
    for (let v = 0; v <= top; v += step) {
      svg("line", { class: v ? "grid" : "axis", x1: m.left, x2: W - m.right, y1: y(v), y2: y(v) }, root);
      svg("text", { x: m.left - 8, y: y(v) + 4, "text-anchor": "end" }, root).textContent = fmt(v);
    }
    const every = Math.ceil(days.length / 6);
    days.forEach((d, i) => {
      if (i % every === 0 || i === days.length - 1) {
        svg("text", { x: x(i), y: H - 8, "text-anchor": i === 0 ? "start" : i === days.length - 1 ? "end" : "middle" }, root).textContent = d.slice(8, 10) + "/" + d.slice(5, 7);
      }
    });
    for (const s of series) {
      const points = s.values.map((v, i) => `${x(i)},${y(v)}`).join(" ");
      svg("polyline", { points, fill: "none", stroke: s.color, "stroke-width": 2, "stroke-linejoin": "round", "stroke-linecap": "round" }, root);
    }
    const cross = svg("line", { class: "cross", y1: m.top, y2: m.top + ih, visibility: "hidden" }, root);
    const dots = series.map((s) => svg("circle", { r: 4, fill: s.color, stroke: "var(--surface-1)", "stroke-width": 2, visibility: "hidden" }, root));
    const hit = svg("rect", { class: "hit", x: m.left, y: m.top, width: iw, height: ih }, root);
    hit.addEventListener("pointermove", (e) => {
      const box = root.getBoundingClientRect();
      const px = ((e.clientX - box.left) / box.width) * W;
      const i = Math.max(0, Math.min(days.length - 1, Math.round(((px - m.left) / iw) * (days.length - 1))));
      cross.setAttribute("x1", x(i));
      cross.setAttribute("x2", x(i));
      cross.setAttribute("visibility", "visible");
      dots.forEach((dot, k) => {
        dot.setAttribute("cx", x(i));
        dot.setAttribute("cy", y(series[k].values[i]));
        dot.setAttribute("visibility", "visible");
      });
      showTip(`<b>${dateVi(days[i])}</b>` + series.map((s) => `<div class="row"><i class="sw ${s.cls}"></i>${s.name} <b>${fmt(s.values[i])}</b></div>`).join(""), e);
    });
    hit.addEventListener("pointerleave", () => {
      cross.setAttribute("visibility", "hidden");
      dots.forEach((dot) => dot.setAttribute("visibility", "hidden"));
      hideTip();
    });
    container.replaceChildren(root);
  }

  function barList(container, rows, label) {
    container.replaceChildren();
    const total = rows.reduce((sum, r) => sum + r.n, 0);
    if (!rows.length) {
      container.textContent = "Chưa có dữ liệu.";
      return;
    }
    for (const r of rows) {
      const row = document.createElement("div");
      row.className = "bar-row";
      const name = document.createElement("span");
      name.textContent = label(r);
      const track = document.createElement("span");
      track.className = "track";
      const fill = document.createElement("span");
      fill.className = "fill";
      fill.style.width = total ? `${(r.n / total) * 100}%` : "0";
      fill.style.display = "block";
      track.append(fill);
      const n = document.createElement("b");
      n.textContent = `${fmt(r.n)} · ${pct(r.n, total)}`;
      row.append(name, track, n);
      container.append(row);
    }
  }

  // --------------------------------------------------------- downloads
  async function loadDownloads() {
    const tiles = $("dl-tiles");
    tiles.setAttribute("aria-busy", "true");
    tiles.querySelectorAll("[data-k]").forEach((el) => (el.textContent = ""));
    $("dl-state").hidden = true;

    const releases = [];
    for (let page = 1; ; page++) {
      const response = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=100&page=${page}`, { cache: "no-store" });
      if (!response.ok) throw new Error(response.status === 403 ? "GitHub tạm giới hạn số lần truy cập, thử lại sau ít phút." : `GitHub trả lỗi ${response.status}.`);
      const batch = await response.json();
      releases.push(...batch);
      if (batch.length < 100) break;
    }
    const published = releases.filter((r) => !r.draft).sort((a, b) => new Date(a.published_at) - new Date(b.published_at));
    const items = published.map((r) => ({
      tag: r.tag_name,
      date: dateVi(r.published_at),
      url: r.html_url,
      win: r.assets.filter((a) => isWindows(a.name)).reduce((s, a) => s + a.download_count, 0),
      mac: r.assets.filter((a) => isMac(a.name)).reduce((s, a) => s + a.download_count, 0),
    }));

    const win = items.reduce((s, d) => s + d.win, 0);
    const mac = items.reduce((s, d) => s + d.mac, 0);
    const total = win + mac;
    const latest = items[items.length - 1];
    setText(tiles, "total", fmt(total));
    setText(tiles, "total-meta", `qua ${items.length} phiên bản`);
    setText(tiles, "latest", latest ? fmt(latest.win + latest.mac) : "0");
    setText(tiles, "latest-meta", latest ? `${latest.tag} · ${latest.date}` : "chưa phát hành");
    setText(tiles, "win", fmt(win));
    setText(tiles, "win-meta", `${pct(win, total)} tổng lượt tải`);
    setText(tiles, "mac", fmt(mac));
    setText(tiles, "mac-meta", `${pct(mac, total)} tổng lượt tải`);
    tiles.setAttribute("aria-busy", "false");

    if (!items.length) {
      $("dl-chart").replaceChildren();
      $("dl-state").textContent = "Chưa có bản phát hành nào.";
      $("dl-state").hidden = false;
      return;
    }
    const drawDownloads = () => stackedBars($("dl-chart"), items);
    drawDownloads();
    redraws.downloads = drawDownloads;
    table(
      $("dl-table"),
      [["Phiên bản"], ["Ngày phát hành"], ["Windows", 1], ["macOS", 1], ["Tổng", 1]],
      [...items].reverse().map((d) => [d.tag, d.date, fmt(d.win), fmt(d.mac), fmt(d.win + d.mac)]),
    );
  }

  // ------------------------------------------------------------- users
  function usersEmpty(title, text) {
    $("users-live").hidden = true;
    $("users-empty").hidden = false;
    $("users-empty-title").textContent = title;
    $("users-empty-text").textContent = text;
  }

  async function loadUsers() {
    if (!STATS) {
      usersEmpty(
        "Chưa bật đếm người dùng",
        "Số người dùng chỉ có khi máy chủ thống kê đã được triển khai và gắn vào web lẫn bản phát hành của app. Trang không hiển thị số ước đoán.",
      );
      return;
    }
    usersEmpty("Đang tải số người dùng…", "");
    const response = await fetch(`${STATS}/stats?days=30`, { cache: "no-store" });
    if (!response.ok) throw new Error(`Máy chủ thống kê trả lỗi ${response.status}.`);
    const data = await response.json();

    const days = [];
    for (let i = 0; i < data.days; i++) days.push(new Date(Date.parse(data.since) + i * 86400000).toISOString().slice(0, 10));
    const by = Object.fromEntries(days.map((d) => [d, { app: 0, web: 0 }]));
    for (const { day, kind, n } of data.daily) if (by[day]) by[day][kind] = n;
    const totals = Object.fromEntries(data.totals.map((t) => [t.kind, t]));
    const live = $("users-live");
    setText(live, "app-today", fmt(by[days[days.length - 1]].app));
    setText(live, "app-30", fmt(totals.app?.period));
    setText(live, "app-all", fmt(totals.app?.total));
    setText(live, "web-30", fmt(totals.web?.period));

    $("users-empty").hidden = true;
    live.hidden = false;
    const series = [
      { name: "App", cls: "s1", color: "var(--series-1)", values: days.map((d) => by[d].app) },
      { name: "Web", cls: "s3", color: "var(--series-3)", values: days.map((d) => by[d].web) },
    ];
    const drawUsers = () => lineChart($("users-chart"), days, series);
    drawUsers();
    redraws.users = drawUsers;
    table(
      $("users-table"),
      [["Ngày"], ["App", 1], ["Web", 1]],
      [...days].reverse().map((d) => [dateVi(d), fmt(by[d].app), fmt(by[d].web)]),
    );
    barList($("versions"), data.versions, (r) => (r.version ? `v${r.version}` : "Không rõ"));
    const OS = { windows: "Windows", macos: "macOS", linux: "Linux" };
    barList($("systems"), data.systems, (r) => OS[r.os] || r.os || "Không rõ");
  }

  // -------------------------------------------------------------- run
  async function load() {
    const button = $("refresh");
    button.disabled = true;
    const results = await Promise.allSettled([loadDownloads(), loadUsers()]);
    if (results[0].status === "fulfilled") {
      const isEn = window.SwipelateI18n?.getActiveLang?.() === "en";
      $("updated").textContent = isEn
        ? `Updated at ${new Date().toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit" })}.`
        : `Cập nhật lúc ${new Date().toLocaleTimeString("vi-VN", { hour: "2-digit", minute: "2-digit" })}.`;
    } else {
      // Old numbers would look current, so clear them.
      $("dl-tiles").querySelectorAll("[data-k]").forEach((el) => (el.textContent = "–"));
      $("dl-chart").replaceChildren();
      $("dl-table").replaceChildren();
      delete redraws.downloads;
      $("dl-tiles").setAttribute("aria-busy", "false");
      const isEn = window.SwipelateI18n?.getActiveLang?.() === "en";
      $("dl-state").textContent = isEn
        ? `Failed to load downloads: ${results[0].reason.message}`
        : `Không tải được lượt tải: ${results[0].reason.message}`;
      $("dl-state").hidden = false;
    }
    if (results[1].status === "rejected") {
      const isEn = window.SwipelateI18n?.getActiveLang?.() === "en";
      usersEmpty(isEn ? "Cannot connect to stats server" : "Không kết nối được máy chủ thống kê", results[1].reason.message);
    }
    button.disabled = false;
  }

  // Charts are drawn at the container's real width, so redraw on resize.
  let resizing;
  addEventListener("resize", () => {
    clearTimeout(resizing);
    resizing = setTimeout(() => Object.values(redraws).forEach((draw) => draw()), 200);
  });

  window.addEventListener("swipelate-lang-changed", () => {
    document.querySelectorAll("[data-table]").forEach((button) => {
      const id = button.dataset.table;
      const tableEl = $(`${id}-table`);
      const showTable = !tableEl.hidden;
      const isEn = window.SwipelateI18n?.getActiveLang?.() === "en";
      button.textContent = showTable
        ? (isEn ? "View as chart" : "Xem dạng biểu đồ")
        : (isEn ? "View as table" : "Xem dạng bảng");
    });
    Object.values(redraws).forEach((draw) => draw());
  });

  $("refresh").addEventListener("click", load);
  load();
})();
