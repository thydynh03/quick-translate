// Thống kê ẩn danh cho Quick Translate: app và web gửi POST /ping, dashboard đọc GET /stats.
const CORS = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET, POST, OPTIONS",
  "access-control-allow-headers": "content-type",
};
const KINDS = new Set(["app", "web"]);
const ID = /^[A-Za-z0-9-]{8,64}$/;
const clip = (value) => String(value ?? "").replace(/[^\w.+-]/g, "").slice(0, 24);

const json = (body, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { ...CORS, "content-type": "application/json" } });

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (request.method === "OPTIONS") return new Response(null, { headers: CORS });

    if (url.pathname === "/ping" && request.method === "POST") {
      const body = await request.json().catch(() => null);
      if (!body || !KINDS.has(body.kind) || !ID.test(body.id ?? "")) return json({ error: "bad request" }, 400);
      const day = new Date().toISOString().slice(0, 10);
      await env.DB.prepare("INSERT OR IGNORE INTO hits (day, kind, id, version, os) VALUES (?, ?, ?, ?, ?)")
        .bind(day, body.kind, body.id, clip(body.version), clip(body.os))
        .run();
      return json({ ok: true });
    }

    if (url.pathname === "/stats" && request.method === "GET") {
      const days = Math.min(Math.max(Math.trunc(Number(url.searchParams.get("days"))) || 30, 1), 365);
      const since = new Date(Date.now() - (days - 1) * 86400000).toISOString().slice(0, 10);
      const [daily, totals, versions, systems] = await env.DB.batch([
        env.DB.prepare("SELECT day, kind, COUNT(*) AS n FROM hits WHERE day >= ? GROUP BY day, kind ORDER BY day").bind(since),
        env.DB.prepare(
          "SELECT kind, COUNT(DISTINCT id) AS total, COUNT(DISTINCT CASE WHEN day >= ? THEN id END) AS period FROM hits GROUP BY kind",
        ).bind(since),
        env.DB.prepare(
          // Each machine counts once, under the version of its latest ping.
          "SELECT version, COUNT(*) AS n FROM hits h WHERE kind = 'app' AND day >= ? AND day = (SELECT MAX(day) FROM hits WHERE kind = 'app' AND id = h.id) GROUP BY version ORDER BY n DESC",
        ).bind(since),
        env.DB.prepare("SELECT os, COUNT(DISTINCT id) AS n FROM hits WHERE kind = 'app' AND day >= ? GROUP BY os ORDER BY n DESC").bind(
          since,
        ),
      ]);
      return json({
        days,
        since,
        daily: daily.results,
        totals: totals.results,
        versions: versions.results,
        systems: systems.results,
      });
    }

    return json({ error: "not found" }, 404);
  },
};
