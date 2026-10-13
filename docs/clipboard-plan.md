# Kế hoạch: bảng clipboard (lịch sử copy)

Trạng thái: **đã làm** trên nhánh `feat/clipboard-board` (2026-10-13). Khác với bản kế hoạch: theo ý người dùng, làm giống Win+V nên chỉ mục ghim được lưu xuống đĩa, giữ 50 mục, giao diện kính mờ kiểu macOS, chưa có mục trong menu khay.

## Mục tiêu

macOS không có bảng lịch sử copy như Win+V của Windows. Thêm vào Quick Translate: bấm phím tắt → hiện danh sách những đoạn đã copy → chọn một đoạn để dán vào ứng dụng đang dùng.

## Phạm vi bản đầu

- Chỉ lưu **văn bản** (ảnh, file để bản sau).
- Bật mặc định trên macOS. Trên Windows code vẫn chạy được nhưng phím tắt để trống (đã có Win+V); lý do giữ cho Windows: máy phát triển là Windows, đây là cách duy nhất để thử giao diện và logic trước khi phát hành.
- Giữ 100 mục gần nhất, mỗi mục tối đa 100 KB. Copy lại một đoạn cũ thì đưa nó lên đầu, không tạo bản trùng.
- Ghim mục hay dùng (mục ghim không bị đẩy ra khi đầy).

## Cách hoạt động

1. **Theo dõi**: một luồng nền hỏi số đếm thay đổi của clipboard mỗi 300 ms (`NSPasteboard.changeCount` trên macOS, `GetClipboardSequenceNumber` trên Windows). macOS không có sự kiện báo clipboard đổi nên phải hỏi định kỳ; phép hỏi này rất nhẹ.
2. **Bỏ qua**:
   - Mục mà trình quản lý mật khẩu đánh dấu bí mật (`org.nspasteboard.ConcealedType`, `TransientType`).
   - Lần copy do chính app tạo ra khi đọc vùng bôi đen (`selection_via_copy` bấm Cmd+C rồi trả clipboard cũ về).
   - Ứng dụng nằm trong danh sách loại trừ sẵn có (`excluded_apps`).
3. **Lưu**: `clipboard.json` trong thư mục dữ liệu của app. Có công tắc "giữ lại sau khi khởi động lại" và nút "Xoá hết".
4. **Hiện bảng**: cửa sổ mới `clipboard`, hiện gần con trỏ, không cướp tiêu điểm của ứng dụng đang dùng (cùng kiểu cửa sổ với popup dịch).
   - Ô tìm kiếm, phím lên/xuống + Enter, phím 1–9 chọn nhanh, Esc đóng, bấm ra ngoài thì đóng.
   - Mỗi mục có: dán, ghim, xoá, và **Dịch** (mở popup dịch với đoạn đó).
5. **Dán**: ghi đoạn được chọn vào clipboard → ẩn bảng → gửi Cmd+V (macOS) / Ctrl+V (Windows). Trên macOS dùng quyền Accessibility mà app đã xin sẵn.

## Phím tắt mặc định

- macOS: `Cmd+Shift+V`.
- Windows: để trống.
- Đổi được trong Cài đặt như các phím tắt khác.

## File sẽ đụng tới

| File | Việc |
|---|---|
| `src-tauri/src/clipboard.rs` (mới) | Kho lưu, luồng theo dõi, cửa sổ, các lệnh `clipboard_list/paste/pin/remove/clear` |
| `src-tauri/src/platform/macos.rs`, `windows.rs` | `clipboard_change()`, `clipboard_concealed()`, `send_paste()` |
| `src-tauri/src/settings.rs`, `src/types.ts` | `clipboard_hotkey`, `clipboard_history`, `clipboard_keep` |
| `src-tauri/src/lib.rs` | Đăng ký phím tắt, lệnh, mục trong menu khay |
| `src/Clipboard.tsx` (mới), `src/main.tsx`, `src/styles.css` | Giao diện bảng |
| `src/Settings.tsx` | Công tắc, phím tắt, nút xoá |
| `src-tauri/capabilities/default.json` | Thêm cửa sổ `clipboard` |
| `website/docs.html`, `README.md` | Hướng dẫn |

## Các mốc

1. **M1** Kho lưu + theo dõi + unit test (trùng lặp, giới hạn, ghim, bỏ mục bí mật).
2. **M2** Cửa sổ bảng + dán + nút Dịch. Thử tay trên Windows.
3. **M3** Cài đặt, menu khay, tài liệu. PR → CI (Windows + macOS) → phát hành 0.1.19.

## Rủi ro và giới hạn

- **Không có máy Mac để thử.** CI chỉ chứng minh code macOS biên dịch được. Ba chỗ chỉ thử được trên Mac thật: cửa sổ nhận phím mà không cướp tiêu điểm, Cmd+V dán đúng ứng dụng, và nhận diện mục bí mật. Cần một người dùng Mac thử trước khi coi là xong.
- Lịch sử copy có thể chứa thông tin nhạy cảm. Vì vậy: bỏ mục bí mật, có danh sách loại trừ, có nút xoá, file chỉ nằm trên máy và không gửi đi đâu.
- `Cmd+Shift+V` là "dán không định dạng" ở một số ứng dụng (Chrome, Google Docs); phím tắt toàn cục sẽ chiếm nó. Người dùng đổi được.

## Cần người dùng quyết định

1. Lưu lịch sử xuống đĩa mặc định (đề xuất: có), hay chỉ giữ trong bộ nhớ?
2. Phím tắt mặc định trên macOS: `Cmd+Shift+V` (đề xuất) hay phím khác, ví dụ `Alt+V`?
3. Có bật sẵn trên Windows không (đề xuất: không)?
