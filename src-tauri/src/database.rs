use std::path::{Path, PathBuf};
use rusqlite::{Connection, Result, Error};
use rusqlite_migration::{Migrations, M};
use std::collections::HashSet;
use lazy_static::lazy_static;

lazy_static! {
  static ref MIGRATIONS: Migrations<'static> =
    Migrations::new(vec![
      M::up(include_str!("migrations/01u_inital.sql"))
      .down(include_str!("migrations/01d_inital.sql")),
    ]);
}

fn create_connection(path: Option<&Path>) -> Result<Connection, Error> {
  let conn = match path {
    Some(path) => Connection::open(path)?,
    None => Connection::open_in_memory()?,
  };
  conn.pragma_update(None, "journal_mode", "WAL").unwrap();
  conn.pragma_update(None, "foreign_keys", "ON").unwrap();
  Ok(conn)
}

#[derive(Debug)]
struct Item {
  path: PathBuf,
  tags: HashSet<String>,
}

impl Item {
  fn path_sql(&self) -> String {
    self.path.as_os_str().to_str().unwrap().into()
  }

  fn tags_sql(&self) -> String {
    let mut tags: Vec<String> = self.tags.iter().map(|x| x.to_owned()).collect();
    tags.sort();
    tags.join(" ")
  }

  fn parse_tags(tags: &str) -> HashSet<String> {
    tags.split_whitespace().map(|t| t.to_owned()).collect()
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;
  use indoc::indoc;
  use super::*;

  #[test]
  fn asd() {
    let mut conn = create_connection(None).unwrap();

    MIGRATIONS.to_latest(&mut conn).unwrap();
    MIGRATIONS.to_version(&mut conn, 0).unwrap();

    let mut stmt = conn.prepare(r"SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name").unwrap();
    let rows: Vec<String> = stmt.query_map([], |row| row.get(0)).unwrap().flatten().collect();
    dbg!(rows);
  }

  #[test]
  fn it_works() -> Result<()> {
    let conn = create_connection(None)?;

    conn.execute(indoc! {"
      CREATE TABLE items (
        path TEXT NOT NULL,
        tags TEXT NOT NULL
      )
      "},
      (),
    )?;

    let item = Item {
      path: PathBuf::from("pixiv/12627130 srgrafo/98705331_p0.jpg"),
      tags: Item::parse_tags("1girl black_hair blush blue_eyes"),
    };

    conn.execute(
      "INSERT INTO items (path, tags) VALUES (?1, ?2)",
      (item.path_sql(), item.tags_sql()),
    )?;

    let mut stmt = conn.prepare("SELECT path, tags FROM items")?;
    let item_iter = stmt.query_map([], |row| {
      Ok(Item {
        path: PathBuf::from(row.get::<usize, String>(0)?),
        tags: Item::parse_tags(&row.get::<usize, String>(1)?),
      })
    })?;

    for item in item_iter {
      println!("Found item {:?}", item.unwrap());
    }
    Ok(())
  }
}
