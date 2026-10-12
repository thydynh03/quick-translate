// Thống kê ẩn danh cho Quick Translate: app và web gửi POST /ping, dashboard đọc GET /stats.
const CORS = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET, POST, OPTIONS",
  "access-control-allow-headers": "content-type",
};
const KINDS = new Set(["app", "web"]);
const ID = /^[A-Za-z0-9-]{8,64}$/;
const clip = (value) => String(value ?? "").replace(/[^\w.+-]/g, "").slice(0, 24);

// Chống số liệu giả: mỗi địa chỉ mạng chỉ được thêm một số mã mới mỗi ngày.
// Một văn phòng hay quán cà phê dùng chung địa chỉ vẫn nằm trong mức này.
const DAILY_NEW_IDS = 30;
const MAX_BODY = 1024;

// Muối của ngày hôm nay, giữ trong bộ nhớ của worker để khỏi đọc lại mỗi lần.
let salt = { day: "", value: "" };

/** Mã băm của địa chỉ gửi, chỉ có nghĩa trong ngày: muối đổi mỗi ngày và dòng cũ bị xoá. */
async function sender(env, day, address) {
  if (salt.day !== day) {
    const fresh = crypto.randomUUID();
    const [, row] = await env.DB.batch([
      env.DB.prepare("INSERT OR IGNORE INTO salts (day, salt) VALUES (?, ?)").bind(day, fresh),
      env.DB.prepare("SELECT salt FROM salts WHERE day = ?").bind(day),
      env.DB.prepare("DELETE FROM salts WHERE day < ?").bind(day),
      env.DB.prepare("DELETE FROM senders WHERE day < ?").bind(day),
    ]);
    salt = { day, value: row.results[0].salt };
  }
  const bytes = new TextEncoder().encode(`${salt.value}|${address}`);
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return [...digest.slice(0, 12)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

const json = (body, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { ...CORS, "content-type": "application/json" } });

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (request.method === "OPTIONS") return new Response(null, { headers: CORS });

    if (url.pathname === "/ping" && request.method === "POST") {
      const address = request.headers.get("cf-connecting-ip") || "unknown";
      // Chặn gửi dồn dập trước khi đụng tới cơ sở dữ liệu.
      if (env.PING_LIMITER && !(await env.PING_LIMITER.limit({ key: address })).success) {
        return json({ error: "too many requests" }, 429);
      }
      if (Number(request.headers.get("content-length")) > MAX_BODY) return json({ error: "bad request" }, 400);
      const text = await request.text();
      let body = null;
      try {
        body = new TextEncoder().encode(text).length <= MAX_BODY ? JSON.parse(text) : null;
      } catch {}
      if (!body || !KINDS.has(body.kind) || !ID.test(body.id ?? "")) return json({ error: "bad request" }, 400);
      const day = new Date().toISOString().slice(0, 10);

      // Máy đã được tính hôm nay thì không tốn lượt.
      const known = await env.DB.prepare("SELECT 1 FROM hits WHERE day = ? AND kind = ? AND id = ?")
        .bind(day, body.kind, body.id)
        .first();
      if (known) return json({ ok: true });

      const from = await sender(env, day, address);
      const used = await env.DB.prepare(
        "INSERT INTO senders (day, sender, n) VALUES (?, ?, 1) ON CONFLICT (day, sender) DO UPDATE SET n = n + 1 RETURNING n",
      )
        .bind(day, from)
        .first();
      if (used.n > DAILY_NEW_IDS) return json({ error: "too many requests" }, 429);

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
