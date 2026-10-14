# Plan: favicon mới và giao diện web theo logo mới

Repo: `E:\Templates\quick-translate` (GitHub `thydynh03/swipelate`). Website tĩnh trong `website/`, deploy tự động lên https://swipelate.com/ bằng `.github/workflows/pages.yml` khi merge vào `main`. Đọc `AGENTS.md` trước.

## 1. Sửa favicon cũ
Tab trình duyệt vẫn hiện logo cũ (chữ S hồng) vì `website/favicon.ico` chưa được tạo lại.
- Tạo lại `website/favicon.ico` (16, 32, 48 px) từ `website/assets/icon.svg`. Có thể lấy từ `src-tauri/icons/icon.ico` hoặc dùng Pillow.
- Rà mọi `<link rel="icon">`, `apple-touch-icon` và các icon trong `site.webmanifest` / `learn.webmanifest`. Ảnh PNG nào còn logo cũ thì tạo lại từ `icon.svg`.
- Kiểm tra: không còn file nào dùng màu logo cũ (`#ff4fa3`, `#14141f` của chữ S).

## 2. Đổi giao diện web theo logo mới
Logo mới có gradient cam–hồng `#ff5f6d` → `#ffc371`, với các thẻ trắng.
- Màu chính hiện là `#b3175a` (magenta), nằm trong `website/assets/site.css` và `stats.css`, trong `theme-color` ở các file HTML và trong `*.webmanifest`. Đổi sang bảng màu mới:
  - nút và link: `#e8505b`, hoặc đậm hơn nếu chưa đạt tương phản AA trên nền sáng;
  - gradient trang trí: `#ff5f6d` → `#ffc371`;
  - nền dịu: tông kem ấm.
- Giữ chế độ tối. Kiểm tra độ tương phản chữ trên nút ở cả hai chế độ, đạt chuẩn WCAG AA.
- Áp dụng cho mọi trang: `index`, `docs`, `learn`, `license`, `privacy`, `stats`, `404`.
- Nếu `og.png` dùng màu cũ thì làm lại cho hợp tông. Logo trong ảnh đã đổi rồi.
- Không đổi nội dung chữ, cấu trúc trang, URL hay SEO (canonical, sitemap, JSON-LD).

## 3. Kiểm tra và giao
- Mở `website/index.html` bằng một server tĩnh, xem ở 375 px và 1280 px, ở cả chế độ sáng và tối.
- Tạo branch, commit, mở PR vào `main`. CI phải xanh, các comment review của cubic phải được trả lời và resolve. Merge xong thì kiểm tra lại https://swipelate.com/.
- Commit kết thúc bằng `Co-Authored-By: <tên model>`. Cập nhật `docs/memory/PROGRESS.md`.
