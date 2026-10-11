# Quick Translate — Plan

Cập nhật: 2026-10-11

## Mục tiêu

App desktop chạy nền (Windows + macOS). Bôi đen text ở bất kỳ phần mềm nào (Claude desktop, trình duyệt, PDF, Word…) thì bản dịch hiện trong popup nổi ngay tại vùng bôi đen. Việt ↔ Anh làm trước, mở rộng mọi ngôn ngữ sau. Dịch được cả online lẫn offline.

Mẫu tham khảo về UX: extension "Từ điển tiếng Nhật JP" (tudienjp.com) — chỉ chạy trong trình duyệt. Lấy từ đó:

- Bôi đen là tra; double-click chọn 1 từ, triple-click chọn 1 câu.
- Một từ → popup kiểu từ điển (nghĩa, phiên âm, ví dụ). Một câu/đoạn → popup kiểu dịch.
- Bấm vào từ trong popup để xem chi tiết.
- Hover để dịch (làm sau, xem M6).

## Quyết định đã chốt

| Vấn đề | Quyết định |
|---|---|
| Nền tảng | Windows + macOS |
| Engine dịch | Cả online và offline, người dùng chọn mặc định, tự chuyển offline khi mất mạng |
| Stack | Tauri 2: lõi Rust (hook, lấy text, engine) + UI web (React + TypeScript + Vite) |
| Cặp ngôn ngữ đầu tiên | vi ↔ en, tự nhận diện chiều dịch |

Lý do chọn Tauri thay vì C#/WPF: cần macOS. Lõi Rust gọi được API hệ thống của cả hai OS, UI viết một lần.

## Kiến trúc

```
src-tauri/ (Rust)
  trigger/     hook chuột + phím tắt toàn cục
  selection/   lấy text + toạ độ vùng chọn (theo từng OS)
  engine/      trait Translator: online/*, offline/*
  dict/        từ điển offline cho tra 1 từ
  ocr/         OCR vùng màn hình (theo từng OS)
  store/       cache, lịch sử, cài đặt (SQLite)
src/ (React)
  popup/       cửa sổ overlay
  settings/    cài đặt, lịch sử, quản lý model
```

### Lấy text bôi đen (theo thứ tự ưu tiên)

| Bước | Windows | macOS |
|---|---|---|
| 1. API trợ năng (có toạ độ vùng chọn) | UI Automation `TextPattern.GetSelection` + `GetBoundingRectangles` | Accessibility `AXSelectedText` + `AXBoundsForRange` |
| 2. Giả lập copy, rồi khôi phục clipboard | `Ctrl+C` | `Cmd+C` |
| 3. OCR vùng khoanh | `Windows.Media.Ocr` | Vision framework |

Bước 2 không có toạ độ → popup đặt theo con trỏ chuột.

### Overlay

Cửa sổ Tauri không viền, trong suốt, luôn nổi, không cướp focus (Windows: `WS_EX_NOACTIVATE`; macOS: `NSPanel` non-activating). Đặt đè/sát dưới vùng chọn, tự lật lên trên khi sát mép màn hình. `Esc` hoặc click ngoài để tắt. Nút: copy, ghim, phát âm, đổi engine.

### Engine dịch

- Online: Google Translate, DeepL, LLM (Claude Haiku) — người dùng tự nhập API key trong cài đặt.
- Offline: model dịch máy chạy bằng CTranslate2 (ứng viên: NLLB-200 distilled, Opus-MT vi↔en). Model tải về lần đầu, không đóng gói trong bộ cài.
- Tra 1 từ: từ điển offline Anh–Việt / Việt–Anh (cần tìm nguồn dữ liệu có giấy phép mở).

## Milestone (mỗi cái một phiên)

| # | Nội dung | Xong khi |
|---|---|---|
| M0 | Cài Rust + Tauri CLI, scaffold dự án, app khay hệ thống chạy được | `pnpm tauri dev` mở được app trên Windows |
| M1 | Phím tắt → giả lập copy → dịch online → popup tại con trỏ | Bôi đen trong Claude desktop, bấm phím tắt, thấy bản dịch |
| M2 | UI Automation lấy text + toạ độ; overlay đè vùng chọn; tự kích hoạt khi bôi đen (hiện icon nhỏ, bấm mới dịch) | Popup nằm đúng vùng chọn trong Notepad, Word, Chrome |
| M3 | Engine offline + màn hình tải model; đo tốc độ và chất lượng vi↔en | Rút mạng vẫn dịch được |
| M4 | Chế độ từ điển cho 1 từ (nghĩa, phiên âm, ví dụ, phát âm) | Double-click 1 từ ra popup từ điển |
| M5 | OCR vùng màn hình | Dịch được chữ trong ảnh / PDF scan |
| M6 | Hover để dịch (lấy text dưới con trỏ qua API trợ năng) | Chạy được ở app hỗ trợ, tắt mặc định |
| M7 | Port macOS: quyền Accessibility, NSPanel, Vision OCR, phím tắt | Ngang M1–M5 trên macOS |
| M8 | Cài đặt, lịch sử, danh sách app loại trừ, khởi động cùng máy, bộ cài, ký số | Có file cài cho cả hai OS |

Code theo từng OS đặt sau trait ngay từ M1 để M7 chỉ là viết thêm implementation, không sửa kiến trúc.

## Rủi ro

- **App Electron/Chromium (kể cả Claude desktop):** API trợ năng có lúc không trả vùng chọn → rơi về giả lập copy, popup theo con trỏ.
- **Clipboard:** giả lập copy ghi đè clipboard tạm thời; phải khôi phục đủ định dạng và bỏ qua ô mật khẩu.
- **App chạy quyền admin (Windows):** không hook được trừ khi app dịch cũng chạy admin.
- **macOS:** cần người dùng cấp quyền Accessibility và Screen Recording; build và test phải có máy Mac; phát hành cần Apple Developer ID (có phí) để ký và notarize.
- **Offline:** model khoảng 0,5–1,5 GB; chất lượng câu dài/chuyên ngành thường kém online. Chốt model sau khi đo ở M3.
- **Google Translate không key:** endpoint không chính thức, có thể bị chặn; chỉ dùng làm mặc định tạm, khuyến nghị nhập key.
- **Thư viện Rust cần kiểm chứng ở M0/M3:** crate lấy text bôi đen đa nền tảng, crate hook chuột toàn cục, binding CTranslate2.

## Môi trường máy hiện tại (2026-10-11)

- Có: Node 24.18, pnpm 9.15, npm 11.
- Thiếu: Rust (`rustc`, `cargo`) — cài ở M0 qua rustup, kèm MSVC Build Tools nếu chưa có.
