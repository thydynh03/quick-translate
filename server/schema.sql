-- Một dòng cho mỗi máy/trình duyệt mỗi ngày. Không lưu IP.
CREATE TABLE IF NOT EXISTS hits (
  day TEXT NOT NULL,       -- YYYY-MM-DD (UTC)
  kind TEXT NOT NULL,      -- 'app' | 'web'
  id TEXT NOT NULL,        -- mã ngẫu nhiên, không gắn với người dùng
  version TEXT NOT NULL DEFAULT '',
  os TEXT NOT NULL DEFAULT '',
  PRIMARY KEY (day, kind, id)
);
CREATE INDEX IF NOT EXISTS hits_kind_id ON hits (kind, id);

-- Giới hạn số mã mới mỗi địa chỉ mạng mỗi ngày. `sender` là mã băm của địa chỉ
-- trộn với muối ngẫu nhiên của ngày đó; cả hai bảng tự xoá dòng của ngày cũ.
CREATE TABLE IF NOT EXISTS senders (
  day TEXT NOT NULL,
  sender TEXT NOT NULL,
  n INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (day, sender)
);
CREATE TABLE IF NOT EXISTS salts (
  day TEXT PRIMARY KEY,
  salt TEXT NOT NULL
);
