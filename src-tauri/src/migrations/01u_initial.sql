CREATE TABLE items (
  id INTEGER PRIMARY KEY,
  path TEXT UNIQUE NOT NULL,
  tags TEXT NOT NULL,
  meta_tags TEXT NOT NULL DEFAULT 'all'
);

-- FTS5 Documentation:
-- https://www.sqlite.org/fts5.html
CREATE VIRTUAL TABLE tag_query USING fts5 (
  -- Include columns to be stored on this virtual table:
  -- Include the `id` column so I can join it to `items`, but don't index with FTS
  id UNINDEXED,
  -- Include the `tags` column to index them
  tags,
  -- a 'meta' column that stores additional tags, e.g. 'all'
  meta_tags,

  -- Make this an external content table (don't store the data in this table, but reference
  -- the original table)
  content=items,
  content_rowid=id,

  -- Use the Unicode61 tokenizer
  -- https://www.sqlite.org/fts5.html#unicode61_tokenizer
  tokenize="unicode61"
);

CREATE TRIGGER items_trigger_ai AFTER INSERT ON items BEGIN
  INSERT INTO tag_query(id, tags, meta_tags) VALUES (NEW.id, NEW.tags, NEW.meta_tags);
END;

CREATE TRIGGER items_trigger_ad AFTER DELETE ON items BEGIN
  INSERT INTO tag_query(tag_query, id, tags, meta_tags) VALUES('delete', OLD.id, OLD.tags, OLD.meta_tags);
END;

CREATE TRIGGER items_trigger_au AFTER UPDATE ON items BEGIN
  INSERT INTO tag_query(tag_query, id, tags, meta_tags) VALUES('delete', OLD.id, old.tags, old.meta_tags);
  INSERT INTO tag_query(id, tags, meta_tags) VALUES (NEW.id, NEW.tags, NEW.meta_tags);
END;
