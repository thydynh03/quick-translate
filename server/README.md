# Máy chủ thống kê

Đang chạy tại https://quick-translate-stats.request-desk.workers.dev (triển khai 2026-10-12, tài khoản Cloudflare của chủ dự án).

Cloudflare Worker + D1 (gói miễn phí đủ dùng). Đếm số máy dùng app và số trình duyệt vào web mỗi ngày.
Mỗi máy/trình duyệt có một mã ngẫu nhiên; không lưu IP, không lưu nội dung dịch.
Để giới hạn số liệu giả, máy chủ chỉ giữ một mã băm của địa chỉ mạng trong đúng ngày đó (xem "Chống số liệu giả").

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

## Chống số liệu giả

Chạy ngay trong worker, nên dùng được cả ở địa chỉ `workers.dev` (rule của WAF chỉ áp cho tên miền riêng).

- Mỗi địa chỉ mạng gọi `/ping` tối đa 10 lần mỗi phút (`[[ratelimits]]` trong `wrangler.toml`, cộng một bộ đếm trong bộ nhớ worker). Lớp này chỉ chặn được lũ lớn: khi thử thật 40 lần trong 20 giây vẫn lọt, vì yêu cầu rải qua nhiều máy chủ của Cloudflare. Lớp chặn chắc chắn là mức theo ngày bên dưới.
- Địa chỉ IPv6 được tính theo mạng (64 bit đầu), vì một mạng có vô số địa chỉ.
- Mỗi địa chỉ mạng chỉ thêm được 30 mã mới mỗi ngày (`DAILY_NEW_IDS` trong `src/index.js`). Mã đã được tính trong ngày thì gửi lại không tốn lượt.
- Vượt mức thì nhận mã 429. App tự thử lại sau một giờ.
- Bảng `senders` giữ mã băm SHA-256 của địa chỉ trộn với muối ngẫu nhiên của ngày; muối và các dòng này bị xoá khi worker nhận lượt gửi đầu tiên của ngày sau. Địa chỉ IP không được ghi lại.
- Không chặn được người có rất nhiều địa chỉ mạng khác nhau; khi đó cần tên miền riêng và WAF.

Đã triển khai bản cũ thì chạy lại `npx wrangler d1 execute quick-translate-stats --remote --file schema.sql` rồi `npx wrangler deploy`.

## API

- `POST /ping` `{ "kind": "app" | "web", "id": "...", "version": "0.1.6", "os": "windows" }`
  Trả 429 khi vượt giới hạn.
- `GET /stats?days=30` → `daily`, `totals` (theo `kind`: `total` từ trước tới nay, `period` trong khoảng), `versions`, `systems`.
