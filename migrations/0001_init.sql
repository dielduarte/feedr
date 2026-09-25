CREATE TABLE folders (
  id        INTEGER PRIMARY KEY,
  slug      TEXT NOT NULL UNIQUE,
  name      TEXT NOT NULL UNIQUE,
  position  INTEGER NOT NULL
);

CREATE TABLE feeds (
  id             INTEGER PRIMARY KEY,
  slug           TEXT NOT NULL UNIQUE,
  folder_id      INTEGER REFERENCES folders(id) ON DELETE SET NULL,
  position       INTEGER NOT NULL,
  url            TEXT NOT NULL UNIQUE,
  site_url       TEXT,
  title          TEXT NOT NULL,
  custom_title   TEXT,
  etag           TEXT,
  last_modified  TEXT,
  next_fetch_at  INTEGER NOT NULL,
  error_count    INTEGER NOT NULL DEFAULT 0,
  last_error     TEXT
);

CREATE TABLE items (
  id            INTEGER PRIMARY KEY,
  feed_id       INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
  slug          TEXT NOT NULL,
  guid          TEXT NOT NULL,
  url           TEXT,
  title         TEXT,
  author        TEXT,
  content_html  TEXT,
  summary       TEXT,
  published_at  INTEGER NOT NULL,
  fetched_at    INTEGER NOT NULL,
  read_at       INTEGER,
  starred_at    INTEGER,
  UNIQUE (feed_id, guid),
  UNIQUE (feed_id, slug)
);

CREATE INDEX items_timeline ON items(published_at DESC, id DESC);
CREATE INDEX items_unread ON items(feed_id) WHERE read_at IS NULL;
CREATE INDEX feeds_due ON feeds(next_fetch_at);
