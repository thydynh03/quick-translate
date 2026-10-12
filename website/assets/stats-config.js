// Địa chỉ máy chủ thống kê (server/README.md). Để trống thì web không gửi gì và dashboard chỉ hiện lượt tải.
window.QT_STATS_URL = "";

// Đếm lượt truy cập: mỗi trình duyệt một mã ngẫu nhiên, gửi tối đa một lần mỗi ngày.
(() => {
  const url = window.QT_STATS_URL.replace(/\/$/, "");
  if (!url || navigator.doNotTrack === "1") return;
  try {
    const today = new Date().toISOString().slice(0, 10);
    if (localStorage.getItem("qt-ping") === today) return;
    let id = localStorage.getItem("qt-id");
    if (!id) {
      id = crypto.randomUUID();
      localStorage.setItem("qt-id", id);
    }
    fetch(`${url}/ping`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ kind: "web", id, version: "", os: "" }),
      keepalive: true,
    }).then((response) => response.ok && localStorage.setItem("qt-ping", today));
  } catch {}
})();
