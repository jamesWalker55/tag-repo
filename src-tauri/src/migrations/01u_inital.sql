CREATE TABLE items (
  path TEXT UNIQUE NOT NULL PRIMARY KEY,
  tags TEXT NOT NULL
) WITHOUT ROWID;

CREATE VIRTUAL TABLE tag_query USING fts5 (
  id UNINDEXED,
  tags,
  content=items,
  content_rowid=path,
  tokenize="unicode61 tokenchars '#'"
);

CREATE TRIGGER items_trigger_ai AFTER INSERT ON items BEGIN
  INSERT INTO tag_query(rowid, tags) VALUES (NEW.id, NEW.tags);
END;

CREATE TRIGGER items_trigger_ad AFTER DELETE ON items BEGIN
  INSERT INTO tag_query(tag_query, rowid, tags) VALUES('delete', OLD.id, OLD.tags);
END;

CREATE TRIGGER items_trigger_au AFTER UPDATE ON items BEGIN
  INSERT INTO tag_query(tag_query, rowid, tags) VALUES('delete', OLD.id, old.tags);
  INSERT INTO tag_query(rowid, tags) VALUES (NEW.id, NEW.tags);
END;
