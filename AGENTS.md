# AGENTS.md

Quy tắc cho mọi AI làm việc trong repo này. Đọc `docs/memory/PROGRESS.md` trước khi bắt đầu.

## Dự án

Swipelate: app desktop Tauri 2 (Rust + React/TypeScript). Bôi đen văn bản ở bất kỳ ứng dụng nào, bản dịch hiện trong popup tại vùng bôi đen. Windows đầy đủ; macOS thử nghiệm.

## Lệnh

- Cài: `pnpm install`
- Chạy dev: `pnpm tauri dev`
- Kiểm tra kiểu frontend: `pnpm build`
- Test Rust: `cd src-tauri && cargo test`
- Build bộ cài Windows: `pnpm tauri build --bundles nsis`
- Xem web: `python -m http.server 4173 --directory website`

## Quy tắc

- Thứ tự engine là yêu cầu của chủ dự án: Google (mặc định) → offline trên máy → Claude/DeepL khi có key. Không đổi.
- Không cào web từ điển. Chỉ dùng API công khai hoặc mở link.
- Phần riêng hệ điều hành chỉ nằm trong `src-tauri/src/platform/`. Hai file `windows.rs` và `macos.rs` phải có cùng tập hàm public.
- Popup và lớp OCR tạo bằng code trong `setup`, không khai báo trong `tauri.conf.json`.
- Nội dung web (`website/`) phải khớp với tính năng thật. Tính năng chưa có trên macOS thì ghi rõ.
- Tên tệp phát hành cố định (`Swipelate-Setup-x64.exe`, `Swipelate-macOS-AppleSilicon.dmg`): web liên kết tới `releases/latest/download/<tên>`.
- Phát hành: tăng version ở `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, rồi đẩy tag `vX.Y.Z`.
- Commit, push, tạo tag: chỉ khi chủ dự án yêu cầu. Không nhập API key hay secret thay người dùng.
- Lỗi đã gặp ghi vào `docs/memory/LESSONS.md`; quyết định ghi vào `docs/memory/DECISIONS.md`.
