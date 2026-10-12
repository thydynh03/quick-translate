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

## Phát hành (2026-10-12)

- Repo công khai: https://github.com/thydynh03/quick-translate (MIT, commit bằng email noreply).
- Web: https://thydynh03.github.io/quick-translate/ (thư mục `website/`, workflow `website`). Có trang chủ với animation 4 cảnh, `docs.html`, `license.html`, sitemap, robots, JSON-LD, ảnh og.
- Bản v0.1.0: `QuickTranslate-Setup-x64.exe` (7,2 MB), `QuickTranslate-macOS-AppleSilicon.dmg` (9,6 MB), `SHA256SUMS.txt`. Đã tải lại từ link `releases/latest/download/...` và khớp SHA-256.
- Phát hành bản mới: tăng version ở 3 file (xem AGENTS.md), đẩy tag `vX.Y.Z`.
- App: icon mới (`src-tauri/app-icon.svg`), tab Giới thiệu, giấy phép hiện trong bộ cài NSIS.

- v0.1.1 (2026-10-12): khay hệ thống kiểu Unikey — bấm trái bật/tắt (icon xám khi tắt, `icons/tray-off.png`), nhấp đúp mở Cài đặt, chuột phải ra menu; lần chạy đầu tự bật khởi động cùng máy. Đã cài bản CI 0.1.1 lên máy này; CHƯA tự bấm thử icon khay (không tự động hoá được), autostart trên máy này chưa bật vì không phải lần chạy đầu.

- v0.1.2 (2026-10-12): Alt+W đọc được tiếng Việt khi đã tải model `ocr-vi` (VietOCR ONNX, ~147 MB, nút tải nằm cuối danh sách model offline). Đã test thật: chữ tiếng Việt trong Notepad, Windows OCR ra mất dấu còn VietOCR ra đủ dấu (`cargo test reads_the_screen -- --ignored`, cần biến môi trường ghi trong file test). Sai lặt vặt còn lại: đôi khi mất dấu chấm cuối đoạn, sai hoa/thường.

- v0.1.3 (2026-10-12): (1) `Alt+E` dịch cả cửa sổ đang mở (`capture::start_window`): OCR theo dòng, gộp đoạn, dịch 4 luồng, vẽ đè bản dịch; đã test thật với Notepad, vị trí khớp. (2) Cài đặt: cỡ chữ + độ đậm nền popup. (3) Engine `openai` cho mọi API tương thích OpenAI (ChatGPT/Gemini/Ollama/LM Studio) — chỉ có unit test, CHƯA gọi thật. Chưa làm: hồ sơ theo từng ứng dụng, dịch thời gian thực, Microsoft Store.

- v0.1.4 (2026-10-12): hồ sơ theo ứng dụng (`Settings.profiles`, `Settings::for_app`, `crate::effective`): nguồn dịch + ngôn ngữ đích riêng theo tên file chạy; sửa trong Cài đặt → Chung. Đã test thật: hồ sơ notepad.exe → offline, log `window: ... engine offline into vi`. `Alt+E` với Google nay gộp cả cửa sổ vào 1 yêu cầu (tách theo dòng, không khớp số dòng thì dịch từng đoạn) — CHƯA kiểm chứng được vì Google đang trả 429 sau nhiều lần test; lúc đó app rơi về offline đúng thiết kế.

- v0.1.5 (2026-10-12): dịch thời gian thực `Alt+R` (`capture::toggle_live`, cài đặt `live_hotkey`, mục ở menu khay). Vòng lặp ~0,9 s: `platform::fingerprint` bỏ qua khi ảnh không đổi; OCR → đoạn nào xuất hiện 2 lần đọc liên tiếp mới dịch; cache theo chữ nguồn; phát `live-frame` (cả danh sách) khi khác lần trước. Overlay xuyên chuột (`set_ignore_cursor_events`) và bị loại khỏi ảnh chụp (`WDA_EXCLUDEFROMCAPTURE`) để OCR không đọc lại bản dịch. Đã test thật bằng log trên bản debug: bật, 15 đoạn, gõ thêm câu trong Notepad → dịch thêm, khung hình ổn định, tắt. CHƯA nhìn được bằng mắt vì overlay không hiện trong ảnh chụp màn hình; phần vẽ dùng chung với `Alt+E`.

- v0.1.6 (2026-10-12): macOS có OCR + dịch cửa sổ + dịch thời gian thực (`platform/macos.rs`): chụp bằng `CGWindowListCreateImage` (live: `OnScreenBelowWindow` với số cửa sổ overlay để không chụp bản dịch), đọc chữ bằng Vision `VNRecognizeTextRequest` qua `objc2::msg_send!`, cửa sổ phía trước bằng `CGWindowListCopyWindowInfo` (cũng cấp `foreground_app` = tên app viết thường cho hồ sơ), quyền Screen Recording qua `CGPreflight/RequestScreenCaptureAccess`, hiện overlay bằng `orderFrontRegardless`. CHỈ mới biên dịch + test trên CI macOS, CHƯA chạy trên máy Mac thật.

- v0.1.6: thêm cài đặt `instant_translate` ("Bôi đen là dịch ngay"): trong `trigger::on_selection_gesture`, khi accessibility đọc được vùng bôi đen thì gọi thẳng `popup::translate_text`; khi phải dùng Ctrl+C thì vẫn hiện biểu tượng (cố ý, tránh tự gửi Ctrl+C). CHƯA test bằng tay.

- 2026-10-12: worker thống kê (`server/`) có chống số liệu giả ngay trong code: `[[ratelimits]]` 10 lần/phút/địa chỉ cho `/ping`, và tối đa 30 mã mới/địa chỉ/ngày qua bảng `senders` (mã băm địa chỉ + muối theo ngày trong bảng `salts`, tự xoá ngày cũ). Worker vẫn CHƯA được triển khai (cần người dùng `wrangler login`).
- v0.1.8 (2026-10-12): sửa popup vô hình chặn chuột sau khi đóng (`platform::hide_overlay`, dùng cho popup và lớp phủ `Alt+R`); hai ô phím tắt Alt+E/Alt+R trong Cài đặt dùng class `hotkey`; ghi log lý do engine chính lỗi trước khi dùng dự phòng; test `google_answers` (ignored) gọi Google thật. Bản 0.1.8 đã cài: `Alt+R` bật/tắt chạy đúng, lớp phủ ẩn hẳn khi tắt (kiểm bằng IsWindowVisible + log), Google dịch 24 đoạn trong ~2 s; trang stats.html hiện đúng lượt tải. CHƯA nhìn lại hai ô phím tắt bằng mắt. PR #6 (giới hạn /ping) và #7 đã merge. Đã test tay flashcard trong app: lưu/bỏ lưu từ popup, tab Từ vựng, mã QR, xoá. Google bị 429 nhiều giờ sau các lần test Alt+E dồn dập rồi tự hết. Worker thống kê vẫn chưa triển khai (cần người dùng `wrangler login`).

## CHƯA kiểm chứng

- macOS: build + `cargo test` qua trên GitHub Actions (macos-latest), có .dmg ký ad-hoc. CHƯA chạy trên máy Mac thật. Thiếu: OCR (Vision), hover, popup không cướp focus (NSPanel), tên app đang mở. App hỏi quyền Accessibility và chờ được cấp.
- Bộ cài v0.1.0 từ CI: đã cài im lặng trên máy này (2026-10-12), app mở và chạy; chưa chạy lại bộ test thao tác chuột với bản này. App ĐANG được cài trên máy.
- Claude, DeepL, Oxford API: chỉ có unit test parse; chưa gọi thật vì không có key.

## Không làm được / quyết định

Xem `DECISIONS.md`. Cambridge không có API công khai → chỉ mở link. Oxford API trả phí → người dùng tự nhập key.

## Việc tiếp theo

1. Chạy thử trên Mac thật; NSPanel, Vision OCR; bản Intel.
2. Ký số (chứng chỉ Windows, Apple Developer ID + notarize) để hết cảnh báo khi cài.
3. Tự cập nhật trong app (tauri-plugin-updater cần khoá ký).
4. SEO: Search Console đã xác minh (tệp `website/google59c0ec488f146fc5.html`, KHÔNG xoá) và đã gửi `sitemap.xml` ngày 2026-10-12, trạng thái ban đầu "Không thể tìm nạp" — kiểm tra lại sau 1–2 ngày. Tên miền riêng nếu muốn.
5. Từ điển offline lớn hơn; gợi ý tải model ở lần chạy đầu.

## Ghi chú kỹ thuật

- Build test: `pnpm tauri build --debug --no-bundle` → `src-tauri/target/debug/quick-translate.exe`. Cờ: `--hidden`, `--settings`. Biến `QT_GOOGLE_ENDPOINT` trỏ Google sang host khác để test dự phòng.
- Popup và lớp OCR phải tạo bằng code trong `setup`; khai báo nhiều cửa sổ trong tauri.conf.json làm WebView2 lỗi 0x800700AA.
- tokenizer.json của Xenova có normalizer `Precompiled` rỗng → thay bằng NFKC khi nạp (offline.rs `load_tokenizer`).
- Sửa `%APPDATA%\com.quicktranslate.app\settings.json` bằng PowerShell phải ghi KHÔNG BOM.
- Kiểm tra kiểu macos.rs từ Windows: crate tạm chỉ gồm core-graphics/core-foundation/accessibility-sys + stub `tauri`/`arboard`, `cargo check --target aarch64-apple-darwin` (check cả app không được vì objc2 cần trình biên dịch của Apple).
- Script test tay: `scripts/mouse.ps1`, `scripts/shot.ps1`. Log app: `%TEMP%\quick-translate.log`. Dữ liệu app: `%APPDATA%\com.quicktranslate.app\` (đang có model en-vi, vi-en).

- v0.1.7 (2026-10-12): gồm dashboard thống kê + từ vựng/flashcard dưới đây. Biến `QT_STATS_URL` chưa đặt nên bản này CHƯA gửi thống kê.

- Dashboard thống kê (2026-10-12): `website/stats.html` hiện lượt tải theo phiên bản/hệ điều hành (GitHub Releases API, chạy ngay) và người dùng app/khách web theo ngày (cần máy chủ `server/`: Cloudflare Worker + D1, CHƯA triển khai). App: `telemetry.rs` ping ẩn danh mỗi ngày nếu build có `QT_STATS_URL` (biến Actions `vars.QT_STATS_URL`) và `usage_stats` bật (mặc định bật, tắt trong Cài đặt → Chung). Web: `assets/stats-config.js` (điền `window.QT_STATS_URL`). Rust CHƯA build được ở môi trường Linux này — kiểm chứng qua workflow `build`.

- Từ vựng + flashcard (2026-10-12): nút đánh dấu trên popup lưu thẻ (`vocab.rs`, `vocab.json`, tối đa 5000); Cài đặt → Từ vựng: tìm, xoá, mã QR / sao chép liên kết (`src/deck.ts`: JSON → deflate-raw → base64url trong `#d=`). Điện thoại: `website/learn.html` (PWA, `learn-sw.js` cho offline) — học thẻ SM-2 rút gọn, 20 thẻ mới/ngày, trắc nghiệm, đọc to, chuỗi ngày, xuất/nhập. Lịch sử có ô tìm. Đã test trang học bằng Chromium khổ 390px (nhập link, học, trắc nghiệm, quét lại không trùng). CHƯA build Rust ở đây; CHƯA thử quét QR bằng điện thoại thật. Tham khảo HotLingo (hotlingo.vn): có sẵn ở ta gần hết (bôi đen, OCR, nhiều engine, lịch sử); khác biệt chính là lưu từ → đã làm ở đây.
