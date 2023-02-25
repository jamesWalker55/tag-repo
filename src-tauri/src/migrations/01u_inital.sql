CREATE TABLE items (
  path TEXT UNIQUE NOT NULL,
  tags TEXT NOT NULL
);

-- FTS5 Documentation:
-- https://www.sqlite.org/fts5.html
CREATE VIRTUAL TABLE tag_query USING fts5 (
  -- Include columns to be stored on this virtual table
  -- I'm including the rowid column so I can match it to the tags, but don't index using FTS
  item_rowid UNINDEXED,
  -- Include the tags to index them
  item_tags,

  -- Use the Unicode61 tokenizer
  -- https://www.sqlite.org/fts5.html#unicode61_tokenizer
  tokenize="unicode61",

  -- Make this an external content table (don't store the data in this table, but reference
  -- the original table)
  content=items
--   content_rowid=path,
);

CREATE TRIGGER items_trigger_ai AFTER INSERT ON items BEGIN
  INSERT INTO tag_query(item_rowid, item_tags) VALUES (NEW.rowid, NEW.tags);
END;

CREATE TRIGGER items_trigger_ad AFTER DELETE ON items BEGIN
  INSERT INTO tag_query(tag_query, item_rowid, item_tags) VALUES('delete', OLD.rowid, OLD.tags);
END;

CREATE TRIGGER items_trigger_au AFTER UPDATE ON items BEGIN
  INSERT INTO tag_query(tag_query, item_rowid, item_tags) VALUES('delete', OLD.rowid, old.tags);
  INSERT INTO tag_query(item_rowid, item_tags) VALUES (NEW.rowid, NEW.tags);
END;
