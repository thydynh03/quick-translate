# DECISIONS

| Ngày | Quyết định | Lý do |
|---|---|---|
| 2026-10 | Tauri 2 thay vì Electron | Bộ cài khoảng 7 MB, gọi được API hệ điều hành từ Rust |
| 2026-10 | Engine: Google → offline → Claude/DeepL | Yêu cầu của chủ dự án: đa số người dùng không có API key AI |
| 2026-10 | Offline dùng Opus-MT ONNX, tải khi cần | Không cần key; giữ bộ cài nhỏ |
| 2026-10 | Cambridge, Oxford Learner's: chỉ mở link | Cambridge không có API công khai; không cào web |
| 2026-10 | Oxford API: tuỳ chọn, người dùng tự nhập key | API trả phí |
| 2026-10-12 | Giấy phép MIT, repo công khai | Cần công khai để tải từ web và dùng GitHub Pages miễn phí |
| 2026-10-12 | Web tĩnh trong `website/`, GitHub Pages | Không tốn phí, tải nhanh, dễ SEO |
| 2026-10-12 | Bộ cài chưa ký số; macOS ký ad-hoc | Chứng chỉ ký là tài nguyên trả phí của chủ dự án |
| 2026-10-12 | Commit dùng email noreply của GitHub | Không công khai email cá nhân |
