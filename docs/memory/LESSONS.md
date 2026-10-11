# LESSONS

- Khai báo nhiều cửa sổ trong `tauri.conf.json` làm WebView2 lỗi `0x800700AA` lúc khởi động. Tạo popup và lớp OCR bằng code trong `setup`.
- `tokenizer.json` của Xenova có normalizer `Precompiled` rỗng; thay bằng NFKC khi nạp.
- Opus-MT dịch từ đơn lẻ có thể ra chuỗi rỗng: cấm EOS ở token đầu, và tra danh sách từ trước khi gọi model.
- Nhận diện tiếng Việt phải có ít nhất một chữ cái chỉ tiếng Việt có, nếu không tiếng Pháp bị nhận nhầm.
- PowerShell `Set-Content -Encoding utf8` ghi BOM, app không đọc được `settings.json`. Dùng `[IO.File]::WriteAllText`.
- `reqwest` 0.13 không có feature `rustls-tls`; dùng `native-tls`.
- WinRT async trong crate `windows` 0.62: dùng `.join()`, không phải `.get()`.
- Kéo chọn vùng OCR phải dùng pointer events + `setPointerCapture`, nếu không sẽ kẹt khi lớp phủ chưa có focus.
- Bash tool không có cargo trong PATH; `pnpm tauri build` báo "cargo metadata: program not found".
- `preview_start` có thể nhặt `launch.json` của thư mục cha; kiểm tra tên server trả về.
- Windows.Media.Ocr không có tiếng Việt (máy này chỉ có en-US, ja). Quét chữ Việt bằng Alt+W ra chữ mất dấu → dịch sai. Muốn OCR tiếng Việt phải tự kèm engine khác (ví dụ PaddleOCR ONNX qua `ort`).
