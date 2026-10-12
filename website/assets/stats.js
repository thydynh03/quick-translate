(() => {
  const REPO = "thydynh03/quick-translate";
  const $ = (id) => document.getElementById(id);
  const fmt = (n) => Number(n || 0).toLocaleString("vi-VN");
  const set = (id, n) => ($(id).textContent = fmt(n));
  const row = (cells) => {
    const tr = document.createElement("tr");
    cells.forEach(([text, num]) => {
      const td = document.createElement("td");
      td.textContent = text;
      if (num) td.className = "n";
      tr.append(td);
    });
    return tr;
  };

  // --------------------------------------------------------- downloads
  fetch(`https://api.github.com/repos/${REPO}/releases?per_page=100`)
    .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
    .then((releases) => {
      let win = 0, mac = 0, all = 0;
      const body = $("releases");
      for (const release of releases) {
        let w = 0, m = 0, t = 0;
        for (const asset of release.assets) {
          t += asset.download_count;
          if (/\.exe$|\.msi$/i.test(asset.name)) w += asset.download_count;
          if (/\.dmg$/i.test(asset.name)) m += asset.download_count;
        }
        win += w; mac += m; all += t;
        body.append(row([[release.tag_name], [(release.published_at || "").slice(0, 10)], [fmt(w), 1], [fmt(m), 1], [fmt(t), 1]]));
      }
      set("t-downloads", all); set("t-win", win); set("t-mac", mac);
    })
    .catch(() => ($("releases").innerHTML = '<tr><td colspan="5" class="muted">Không đọc được GitHub API.</td></tr>'));

  // ------------------------------------------------------------- users
  const url = (window.QT_STATS_URL || "").replace(/\/$/, "");
  if (!url) {
    $("users-note").textContent = "Chưa cấu hình máy chủ thống kê (xem server/README.md).";
    return;
  }
  fetch(`${url}/stats?days=30`)
    .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
    .then((data) => {
      const byDay = {};
      for (let i = 0; i < data.days; i++) {
        const day = new Date(Date.parse(data.since) + i * 86400000).toISOString().slice(0, 10);
        byDay[day] = { app: 0, web: 0 };
      }
      for (const { day, kind, n } of data.daily) if (byDay[day]) byDay[day][kind] = n;
      const days = Object.keys(byDay);
      const today = byDay[days[days.length - 1]];
      for (const id of ["t-app-total", "t-app-period", "t-web-period"]) set(id, 0);
      set("t-app-today", today.app); set("t-web-today", today.web);
      for (const { kind, total, period } of data.totals) {
        if (kind === "app") { set("t-app-total", total); set("t-app-period", period); }
        if (kind === "web") set("t-web-period", period);
      }
      const max = Math.max(1, ...days.map((d) => byDay[d].app + byDay[d].web));
      const chart = $("chart");
      for (const day of days) {
        const col = document.createElement("div");
        col.title = `${day}: app ${byDay[day].app}, web ${byDay[day].web}`;
        col.setAttribute("role", "listitem");
        col.setAttribute("aria-label", col.title);
        for (const kind of ["app", "web"]) {
          const bar = document.createElement("i");
          bar.className = kind;
          bar.style.height = `${(byDay[day][kind] / max) * 100}%`;
          col.append(bar);
        }
        chart.append(col);
      }
      const body = $("breakdown");
      for (const { version, n } of data.versions) body.append(row([[`v${version || "?"}`], [fmt(n), 1]]));
      for (const { os, n } of data.systems) body.append(row([[os || "?"], [fmt(n), 1]]));
    })
    .catch(() => ($("users-note").textContent = "Không kết nối được máy chủ thống kê."));
})();
