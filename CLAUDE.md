# CLAUDE.md

Quy tắc chung nằm trong `AGENTS.md`. Phần riêng cho Claude Code:

- Trạng thái dự án: `docs/memory/PROGRESS.md`. Cập nhật file đó khi xong một phase.
- Trong Bash tool, thêm `export PATH="$HOME/.cargo/bin:$PATH"` trước lệnh cargo/tauri.
- Kiểm thử thao tác chuột thật trên Windows: `scripts/mouse.ps1`, `scripts/shot.ps1`.
- macOS không build được từ máy Windows; kiểm chứng bằng workflow `build` trên GitHub Actions.
