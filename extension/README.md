# Quick Translate for YouTube

Extension cho Chrome/Edge: hiện phụ đề tiếng Việt cho video YouTube đang phát. Bản dịch của cả video được lấy một lần ngay khi video bắt đầu, nên không có độ trễ khi xem.

## Cài

1. Mở `chrome://extensions` (Edge: `edge://extensions`).
2. Bật **Developer mode**.
3. Bấm **Load unpacked**, chọn thư mục `extension` này.
4. Mở một video YouTube có phụ đề. Phụ đề tiếng Việt màu vàng hiện phía trên phụ đề gốc.

## Cách hoạt động

- YouTube chỉ trả file phụ đề cho trình phát của chính nó (địa chỉ có kèm mã xác thực). Extension theo dõi yêu cầu phụ đề của trình phát rồi xin lại đúng file đó kèm `tlang=vi`, tức bản do YouTube dịch.
- Trình phát chỉ tải phụ đề khi nút CC đang bật, nên extension tự bật CC một lần cho mỗi video.
- Không gửi dữ liệu đi đâu ngoài YouTube. Không cần ứng dụng Quick Translate đang chạy.

## Giới hạn

- Video không có phụ đề (kể cả phụ đề tự động) thì không có gì để dịch.
- Ngôn ngữ đích cố định là tiếng Việt; đổi hằng `TARGET` trong `main.js` nếu cần.
- YouTube đổi cách tải phụ đề thì extension phải sửa theo.
