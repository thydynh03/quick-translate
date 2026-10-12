# Máy chủ thống kê

Cloudflare Worker + D1 (gói miễn phí đủ dùng). Đếm số máy dùng app và số trình duyệt vào web mỗi ngày.
Mỗi máy/trình duyệt có một mã ngẫu nhiên; không lưu IP, không lưu nội dung dịch.

Lượt tải không cần máy chủ: dashboard (`website/stats.html`) đọc thẳng `download_count` từ GitHub Releases API.

## Triển khai (một lần)

```sh
cd server
npx wrangler login
npx wrangler d1 create quick-translate-stats      # chép database_id vào wrangler.toml
npx wrangler d1 execute quick-translate-stats --remote --file schema.sql
npx wrangler deploy                                # in ra https://quick-translate-stats.<tên>.workers.dev
```

Sau đó:

1. `website/assets/stats-config.js`: điền `window.QT_STATS_URL` = địa chỉ worker, rồi đẩy lên (web + dashboard bắt đầu đếm).
2. GitHub → Settings → Secrets and variables → Actions → **Variables**: tạo `QT_STATS_URL` = địa chỉ worker.
   Bản app phát hành sau đó mới gửi ping (địa chỉ được gắn lúc build).

## API

- `POST /ping` `{ "kind": "app" | "web", "id": "...", "version": "0.1.6", "os": "windows" }`
- `GET /stats?days=30` → `daily`, `totals` (theo `kind`: `total` từ trước tới nay, `period` trong khoảng), `versions`, `systems`.
