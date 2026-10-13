# Quick Translate

Ứng dụng desktop chạy nền: bôi đen văn bản ở bất kỳ phần mềm nào, bản dịch hiện trong popup ngay tại vùng bôi đen. Mặc định dịch Anh ↔ Việt.

**Trang giới thiệu và tải về:** https://thydynh03.github.io/quick-translate/ · [Tài liệu](https://thydynh03.github.io/quick-translate/docs.html) · [Giấy phép](LICENSE)

## Tải về

| Hệ điều hành | Tệp | Ghi chú |
|---|---|---|
| Windows 10/11 x64 | [QuickTranslate-Setup-x64.exe](https://github.com/thydynh03/quick-translate/releases/latest/download/QuickTranslate-Setup-x64.exe) | Đầy đủ tính năng |
| macOS 11+ (chip Apple) | [QuickTranslate-macOS-AppleSilicon.dmg](https://github.com/thydynh03/quick-translate/releases/latest/download/QuickTranslate-macOS-AppleSilicon.dmg) | Thử nghiệm: chưa có OCR và hover |

Bộ cài chưa ký số, hệ điều hành sẽ cảnh báo khi mở lần đầu; xem hướng dẫn trong trang tài liệu.

## Cách dùng

| Thao tác | Kết quả |
|---|---|
| Bôi đen (kéo chuột, double-click, triple-click) rồi bấm biểu tượng nhỏ cạnh con trỏ | Dịch vùng bôi đen |
| `Alt+Q` | Dịch vùng bôi đen, không cần bấm biểu tượng |
| `Alt+W` rồi kéo một hình chữ nhật | Dịch chữ trong vùng màn hình (OCR), dùng cho ảnh, PDF scan |
| `Alt+E` | Dịch cả cửa sổ đang mở, bản dịch phủ lên từng đoạn chữ |
| `Alt+R` | Bật/tắt dịch thời gian thực cho toàn màn hình hoặc một vùng tự kéo: bản dịch tự cập nhật khi nội dung thay đổi |
| Bấm một từ trong phần văn bản gốc của popup | Tra riêng từ đó |
| Giữ `Ctrl` và rê chuột lên một từ (bật trong Cài đặt) | Dịch từ dưới con trỏ, chỉ ở ứng dụng hỗ trợ |
| Chuột phải biểu tượng ở khay hệ thống | Cài đặt, bật/tắt tự hiện biểu tượng, thoát |

Tra một từ tiếng Anh thì popup có thêm nghĩa theo từ loại, phiên âm, định nghĩa tiếng Anh và nút mở từ điển Cambridge / Oxford.

Trên macOS còn có **bảng clipboard** (`Cmd+Shift+V`): danh sách các đoạn đã copy để dán lại, ghim hoặc dịch, tương tự `Win+V` của Windows. Trên Windows tính năng này tắt sẵn; bật bằng cách đặt phím tắt trong Cài đặt.

## Engine dịch

1. **Google** (mặc định): không cần key, cần mạng.
2. **Offline**: model Opus-MT chạy trên máy, không cần mạng hay key. Tải model trong Cài đặt → Engine dịch (khoảng 107 MB mỗi chiều). Khi engine chính lỗi (ví dụ mất mạng), ứng dụng tự chuyển sang offline.
3. **Claude, DeepL, AI qua OpenAI API** (ChatGPT, Gemini, Ollama, LM Studio): chỉ dùng khi bạn cấu hình; là dự phòng sau offline.

Offline dịch câu tốt. Từ đơn tiếng Anh được tra trong từ điển Anh–Việt đóng gói sẵn (hơn 107.000 từ từ Wiktionary, CC BY-SA, xem `src-tauri/data/NOTICE.md`; tạo lại bằng `scripts/build-dictionary.py`), kể cả dạng biến đổi như “leveraging”. Từ không có trong từ điển thì chuyển sang nguồn online khi có mạng.

## Phát triển

Cần Node 20+, pnpm, Rust (stable) và trên Windows là MSVC Build Tools + WebView2.

```bash
pnpm install
```

```bash
pnpm tauri dev
```

```bash
pnpm tauri build
```

Test:

```bash
cd src-tauri && cargo test
```

Test dịch offline thật (tải khoảng 210 MB model vào thư mục tạm):

```bash
cd src-tauri && cargo test offline_end_to_end -- --ignored --nocapture
```

## Cấu trúc

- `src-tauri/src/platform/`: phần riêng từng hệ điều hành (đọc vùng bôi đen, hook chuột, cửa sổ nổi, OCR). Windows đầy đủ; macOS mới có phần đọc vùng bôi đen và theo dõi chuột, chưa chạy thử.
- `src-tauri/src/engine/`: các engine dịch và từ điển.
- `src-tauri/src/popup.rs`, `trigger.rs`, `capture.rs`: popup, nhận diện thao tác bôi đen, chọn vùng OCR.
- `src/`: giao diện React (popup, lớp chọn vùng, cài đặt).
- `website/`: trang giới thiệu tĩnh, triển khai lên GitHub Pages bằng workflow `website`.
- `.github/workflows/`: `build` (test trên Windows + macOS), `release` (đẩy tag `v*` để dựng và phát hành bộ cài), `website`.
- `docs/PLAN.md`, `docs/memory/`: kế hoạch, tiến độ, bài học, quyết định.

## Giới hạn hiện tại

- Mới chạy thử thật trên Windows. Bản macOS build và qua test trên GitHub Actions nhưng chưa được chạy thử trên máy Mac thật. Trên macOS, OCR (`Alt+W`, `Alt+E`, `Alt+R`) dùng Vision và cần quyền Screen Recording; chưa có hover.
- Trong app Electron/Chromium, popup đặt theo con trỏ chứ không theo toạ độ chính xác của vùng bôi đen.
- Không hoạt động trên cửa sổ chạy quyền admin trừ khi Quick Translate cũng chạy quyền admin.
- OCR dùng các gói ngôn ngữ OCR đã cài trong Windows.
- API key lưu dạng văn bản thường trong `%APPDATA%\com.quicktranslate.app\settings.json`.

## Giấy phép

MIT, © 2026 thydynh03. Xem [LICENSE](LICENSE). Dữ liệu từ điển Anh–Việt lấy từ Wiktionary (CC BY-SA), xem `src-tauri/data/NOTICE.md`.
