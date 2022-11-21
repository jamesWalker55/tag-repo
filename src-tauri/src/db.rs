use std::path::Path;
use std::str::FromStr;
use rusqlite::{Error, Connection, params, MappedRows, Row};
use rusqlite::Error::InvalidPath;

#[derive(Debug)]
struct Item {
  id: i32,
  path: String,
  tags: String,
}

struct Database {
  con: Connection,
}

impl Database {
  fn open<P>(path: P) -> Result<Database, Error> where P: AsRef<Path> {
    let con = Connection::open(path)?;
    Ok(Database { con })
  }

  fn initialize(&self) -> Result<(), Error> {
    // Create the main items table
    self.con.execute(r#"
      CREATE TABLE items (
        id INTEGER PRIMARY KEY,
        path TEXT NOT NULL UNIQUE,
        tags TEXT NOT NULL DEFAULT ''
      )
    "#, ())?;
    // Create the fts5 table
    self.con.execute(r#"
      CREATE VIRTUAL TABLE items_fts USING fts5 (
        tags,
        content=items,
        content_rowid=id
      );
    "#, ())?;
    // Triggers to keep fts5 table in sync with main table
    self.con.execute(r#"
      CREATE TRIGGER items_ai AFTER INSERT ON items BEGIN
        INSERT INTO items_fts(rowid, tags) VALUES (new.id, new.tags);
      END;
      CREATE TRIGGER items_ad AFTER DELETE ON items BEGIN
        INSERT INTO items_fts(items_fts, rowid, tags) VALUES('delete', old.id, old.tags);
      END;
      CREATE TRIGGER items_au AFTER UPDATE ON items BEGIN
        INSERT INTO items_fts(items_fts, rowid, tags) VALUES('delete', old.id, old.tags);
        INSERT INTO items_fts(rowid, tags) VALUES (new.id, new.tags);
      END;
    "#, ())?;

    Ok(())
  }

  fn insert_item<P>(&self, path: P) -> Result<(), Error> where P: AsRef<Path> {
    let path = path.as_ref();
    let path = path.to_str().ok_or(Error::InvalidPath(path.to_owned()))?;
    self.con.execute(r#"INSERT INTO items (path) VALUES (?1)"#, params![path])?;
    Ok(())
  }

  fn insert_items<P>(&mut self, paths: &[P]) -> Result<(), Error> where P: AsRef<Path> {
    let tx = self.con.transaction()?;

    for path in paths {
      let path = path.as_ref();
      let path = path.to_str().ok_or(Error::InvalidPath(path.to_owned()))?;
      tx.execute("INSERT INTO items (path) VALUES (?1)", params![path])?;
    }

    tx.commit()
  }

  // fn set_tags(&self, path: P, tags: &str) -> Result<(), Error> where P: AsRef<Path> {
  //   let path = path.as_ref();
  //   let path = path.to_str().ok_or(Error::InvalidPath(path.to_owned()))?;
  //   // self.con.execute(r#"INSERT INTO items (path) VALUES (?1)"#, params![path])?;
  //   Ok(())
  // }

  fn find_by_path<P>(&self, path: P) -> Result<Item, Error> where P: AsRef<Path> {
    todo!("Should normalise path before querying it");
    let path = path.as_ref();
    let path = path.to_str().ok_or(InvalidPath(path.to_owned()))?;
    self.con.query_row(
      "SELECT id, path, tags FROM items WHERE path = ?1",
      [path],
      |row| {
        Ok(Item {
          id: row.get(0)?,
          path: row.get(1)?,
          tags: row.get(2)?,
        })
      },
    )
  }

  fn query(&self, query: &str) -> Result<Vec<Item>, Error> {
    let mut stmt = self.con.prepare(r#"
      SELECT i.id, i.path, i.tags
      FROM items_fts fts
      LEFT JOIN items i on fts.rowid = i.id
      WHERE items_fts MATCH ?1;
    "#)?;
    let items: Result<Vec<Item>, Error> = stmt.query_map(params![query], |row| {
      Ok(Item {
        id: row.get(0)?,
        path: row.get(1)?,
        tags: row.get(2)?,
      })
    })?.collect();
    items
  }

  fn connection(&self) -> &Connection {
    &self.con
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // #[test]
  fn it_works() -> Result<(), Error> {
    let conn = Connection::open_in_memory()?;

    // Create the main items table
    conn.execute(r#"
      CREATE TABLE items (
        id INTEGER PRIMARY KEY,
        path TEXT NOT NULL UNIQUE,
        tags TEXT NOT NULL
      )
    "#, ())?;
    // Create the fts5 table
    conn.execute(r#"
      CREATE VIRTUAL TABLE items_fts USING fts5 (
        tags,
        content=items,
        content_rowid=id
      );
    "#, ())?;
    // Triggers to keep fts5 table in sync with main table
    conn.execute(r#"
      CREATE TRIGGER items_ai AFTER INSERT ON items BEGIN
        INSERT INTO items_fts(rowid, tags) VALUES (new.id, new.tags);
      END;
      CREATE TRIGGER items_ad AFTER DELETE ON items BEGIN
        INSERT INTO items_fts(items_fts, rowid, tags) VALUES('delete', old.id, old.tags);
      END;
      CREATE TRIGGER items_au AFTER UPDATE ON items BEGIN
        INSERT INTO items_fts(items_fts, rowid, tags) VALUES('delete', old.id, old.tags);
        INSERT INTO items_fts(rowid, tags) VALUES (new.id, new.tags);
      END;
    "#, ())?;
    // Insert some values
    conn.execute(r#"
      INSERT INTO items (path, tags) VALUES
        ('rhodz/kicks/01.wav', 'drum kick'),
        ('rhodz/snare/01.wav', 'drum snare'),
        ('rhodz/cymbals/01.wav', 'drum crash'),
        ('black octopus/drums/01.wav', 'drum kick'),
        ('black octopus/leads/01.wav', 'synth lead'),
        ('black octopus/bass/01.wav', 'synth bass');
    "#, ())?;

    let mut stmt = conn.prepare(r#"
      SELECT i.id, i.path, i.tags
      FROM items_fts fts
      LEFT JOIN items i on fts.rowid = i.id
      WHERE items_fts MATCH 'kick OR lead';
    "#)?;
    let item_iter = stmt.query_map([], |row| {
      Ok(Item {
        id: row.get(0)?,
        path: row.get(1)?,
        tags: row.get(2)?,
      })
    })?;
    for item in item_iter {
      dbg!(item).expect("can't print item!");
    }

    Ok(())
  }

  #[test]
  fn it_works2() -> Result<(), Error> {
    let mut db = Database::open(":memory:")?;
    db.initialize()?;

    let paths = vec![
      "rhodz/kicks/01.wav",
      "rhodz/snare/01.wav",
      "rhodz/cymbals/01.wav",
      "black octopus/drums/01.wav",
      "black octopus/leads/01.wav",
      "black octopus/bass/01.wav",
    ];
    db.insert_items(&paths)?;

    Ok(())
  }

  #[test]
  fn it_works3() -> Result<(), Error> {
    let mut db = Database::open(":memory:")?;
    db.initialize()?;

    // Insert some values
    db.connection().execute(r#"
      INSERT INTO items (path, tags) VALUES
        ('rhodz/kicks/01.wav', 'drum kick'),
        ('rhodz/snare/01.wav', 'drum snare'),
        ('rhodz/cymbals/01.wav', 'drum crash'),
        ('black octopus/drums/01.wav', 'drum kick'),
        ('black octopus/leads/01.wav', 'synth lead'),
        ('black octopus/bass/01.wav', 'synth bass');
    "#, ())?;

    dbg!(db.query("kick OR lead").unwrap());

    Ok(())
  }
}
