# PROGRESS

Cập nhật: 2026-10-12. Plan gốc: `docs/PLAN.md`. Hướng dẫn dùng: `README.md`.

## Yêu cầu đã chốt với người dùng

- Windows + macOS; dịch online + offline; Anh ↔ Việt trước.
- Thứ tự engine: (1) Google không key là MẶC ĐỊNH → (2) lỗi/mất mạng thì offline trên máy, không cần API → (3) Claude/DeepL chỉ là dự phòng cuối khi có key.
- Từ điển: gắn Oxford/Cambridge nếu được. KHÔNG cào web.

## Đã xong và đã kiểm chứng trên Windows (chạy thật)

- Bôi đen → icon → popup; phím tắt Alt+Q. Notepad (UI Automation, popup đúng dưới vùng chọn) và Claude desktop (Electron: nhận biết qua con trỏ I-beam, bấm icon mới Ctrl+C, clipboard được khôi phục).
- Google online; offline Opus-MT ONNX (`ort` + `tokenizers`, ~107 MB/chiều, câu en↔vi ~0,7–1,5 s); tải model bằng nút trong Cài đặt.
- Dự phòng: Google không kết nối được → offline, có ghi chú trong popup (`cargo test falls_back_to_offline -- --ignored`).
- Từ đơn offline: danh sách từ Anh–Việt đóng gói sẵn (`src-tauri/data/en-vi-enwiktionary.txt`, 7.458 mục, Wiktionary 2015, CC BY-SA 3.0, xem `data/NOTICE.md`) → "book" ra "sách". Từ không có trong danh sách vẫn rơi về model và có thể sai.
- Tra 1 từ tiếng Anh online: nghĩa Việt theo từ loại + phiên âm (Google), định nghĩa Anh (Free Dictionary API), nút mở Cambridge / Oxford Learner's.
- OCR vùng màn hình: Alt+W → kéo chọn → Windows.Media.Ocr → dịch.
- Hover: giữ Ctrl + rê chuột lên 1 từ (tắt mặc định, chỉ app có UI Automation; đã test Notepad).
- Cài đặt 3 tab; lịch sử; chặn mở 2 bản (lần mở thứ 2 hiện Cài đặt); khởi động cùng máy (đã kiểm tra khoá Run trong registry bật/tắt).
- Bộ cài NSIS: `src-tauri/target/release/bundle/nsis/Quick Translate_0.1.0_x64-setup.exe` (7,2 MB). Bản build trước đó đã cài im lặng, chạy, gỡ cài — đều được; bản build cuối (sau khi thêm macos.rs, không đổi code Windows) chưa cài lại. Máy hiện KHÔNG còn bản cài.
- 30 unit test Rust pass + 2 test thật (`--ignored`).

## Đã viết nhưng CHƯA kiểm chứng

- macOS (`platform/macos.rs`): đọc vùng chọn qua Accessibility API, Cmd+C dự phòng, event tap chuột. Chỉ mới kiểm tra kiểu riêng file này với crate thật cho target `aarch64-apple-darwin`; CHƯA build cả app, CHƯA chạy. Thiếu: OCR (Vision), hover, popup không cướp focus (NSPanel), tên app đang mở. Workflow `.github/workflows/build.yml` build Windows + macOS khi đẩy lên GitHub (chưa từng chạy).
- Claude (`claude-haiku-5-5`), DeepL, Oxford Dictionaries API: chỉ có unit test parse; chưa gọi thật vì không có key.

## Không làm được / quyết định

- Cambridge API: cổng dictionary-api.cambridge.org chuyển hướng về trang chủ, cần xin key riêng → chỉ có nút mở web.
- Oxford API: cần app_id/app_key của người dùng; giấy phép thương mại theo trang của họ từ khoảng £5.000/năm/ngôn ngữ.

## Việc tiếp theo nếu làm tiếp

1. Đẩy lên GitHub, xem CI macOS, sửa lỗi build; thử trên máy Mac thật.
2. macOS: NSPanel cho popup, Vision OCR.
3. Lần chạy đầu gợi ý tải model en↔vi; từ điển offline lớn hơn.
4. Ký số bộ cài (Windows SmartScreen sẽ cảnh báo bản chưa ký).

## Ghi chú kỹ thuật

- Build test: `pnpm tauri build --debug --no-bundle` → `src-tauri/target/debug/quick-translate.exe`. Cờ: `--hidden`, `--settings`. Biến `QT_GOOGLE_ENDPOINT` trỏ Google sang host khác để test dự phòng.
- Popup và lớp OCR phải tạo bằng code trong `setup`; khai báo nhiều cửa sổ trong tauri.conf.json làm WebView2 lỗi 0x800700AA.
- tokenizer.json của Xenova có normalizer `Precompiled` rỗng → thay bằng NFKC khi nạp (offline.rs `load_tokenizer`).
- Sửa `%APPDATA%\com.quicktranslate.app\settings.json` bằng PowerShell phải ghi KHÔNG BOM.
- Kiểm tra kiểu macos.rs từ Windows: crate tạm chỉ gồm core-graphics/core-foundation/accessibility-sys + stub `tauri`/`arboard`, `cargo check --target aarch64-apple-darwin` (check cả app không được vì objc2 cần trình biên dịch của Apple).
- Script test tay: `scripts/mouse.ps1`, `scripts/shot.ps1`. Log app: `%TEMP%\quick-translate.log`. Dữ liệu app: `%APPDATA%\com.quicktranslate.app\` (đang có model en-vi, vi-en).
