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
