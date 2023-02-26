use std::fs::create_dir;
use std::path::{Path, PathBuf};
use indoc::indoc;
use rusqlite::{Connection, ErrorCode, Row};
use rusqlite_migration::{Migrations, M};
use lazy_static::lazy_static;
use rusqlite::Error::SqliteFailure;

#[derive(Debug)]
pub enum OpenError {
  PathDoesNotExist,
  FailedToCreateRepo(std::io::Error),
  FailedToCreateDatabase(rusqlite::Error),
  FailedToMigrateDatabase(rusqlite_migration::Error),
}

#[derive(Debug)]
pub enum DatabaseError {
  DuplicatePathError(PathBuf),
  BackendError(rusqlite::Error),
}

impl From<rusqlite::Error> for DatabaseError {
  fn from(error: rusqlite::Error) -> Self {
    DatabaseError::BackendError(error)
  }
}

pub struct Item {
  path: String,
  tags: String,
}

pub struct Repo {
  conn: Connection,
}

impl Repo {
  fn open(repo_path: impl AsRef<Path>) -> Result<Repo, OpenError> {
    let repo_path = repo_path.as_ref();
    if !repo_path.exists() {
      return Err(OpenError::PathDoesNotExist);
    }
    let data_path = repo_path.join(".tagrepo");
    if !data_path.exists() {
      create_dir(&data_path).map_err(OpenError::FailedToCreateRepo)?;
    }
    let db_path = data_path.join("tags.db");
    let conn = open_database(&db_path)?;
    let repo = Self { conn };
    Ok(repo)
  }

  fn insert_item<T>(&self, path: T, tags: T) -> Result<(), DatabaseError>
  where
    T: AsRef<str>,
  {
    let path = path.as_ref();
    let tags = tags.as_ref();
    let result = self.conn.execute(
      "INSERT INTO items (path, tags) VALUES (?1, ?2)",
      (&path, &tags),
    );

    match result {
      Ok(_) => Ok(()),
      Err(SqliteFailure(sqlite_err, Some(msg))) => {
        if sqlite_err.code == ErrorCode::ConstraintViolation
          && msg == "UNIQUE constraint failed: items.path"
        {
          Err(DatabaseError::DuplicatePathError(path.into()))
        } else {
          Err(DatabaseError::BackendError(SqliteFailure(
            sqlite_err,
            Some(msg),
          )))
        }
      }
      Err(err) => Err(DatabaseError::BackendError(err)),
    }
  }

  fn get_items(&self, query: Option<impl AsRef<str>>) -> Result<Vec<Item>, DatabaseError> {
    let to_item_closure: fn(&Row) -> Result<Item, rusqlite::Error> = |row: &Row| {
      Ok(Item { path: row.get::<_, String>(0)?, tags: row.get::<_, String>(1)? })
    };

    let mut stmt;

    let mapped_rows = match query {
      Some(query) => {
        stmt = self.conn.prepare(indoc! {"
          SELECT i.path, i.tags
          FROM items i
            INNER JOIN tag_query tq ON i.id = tq.id
          WHERE tq.tag_query MATCH :query
        "})?;
        stmt.query_map(&[(":query", query.as_ref())], to_item_closure)
      }
      None => {
        stmt = self.conn.prepare(indoc! {"
          SELECT i.path, i.tags FROM items i
        "})?;
        stmt.query_map([], to_item_closure)
      }
    }?;
    let items: Vec<Item> = mapped_rows.flatten().collect();
    Ok(items)
  }
}

lazy_static! {
  static ref MIGRATIONS: Migrations<'static> =
    Migrations::new(vec![
      M::up(include_str!("migrations/01u_inital.sql"))
      .down(include_str!("migrations/01d_inital.sql")),
    ]);
}

pub fn open_database(db_path: impl AsRef<Path>) -> Result<Connection, OpenError> {
  let db_path = db_path.as_ref();
  let mut conn = Connection::open(db_path).map_err(OpenError::FailedToCreateDatabase)?;

  MIGRATIONS
    .to_latest(&mut conn)
    .map_err(OpenError::FailedToMigrateDatabase)?;

  Ok(conn)
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  fn empty_repo() -> Repo {
    Repo::open(tempdir().unwrap()).unwrap()
  }

  fn test_repo() -> Repo {
    let repo = empty_repo();
    repo.insert_item("hello", "text root").unwrap();
    repo.insert_item("hello2", "text root").unwrap();
    repo.insert_item("hello3", "video root").unwrap();
    repo.insert_item("hello4", "text root apple").unwrap();
    repo.insert_item("world", "video root").unwrap();
    repo
  }

  fn unordered_eq<'a, T, U>(a: T, b: U)
  where
    T: Iterator<Item = &'a str>,
    U: Iterator<Item = &'a str>,
  {
    let mut a: Vec<&str> = a.collect();
    let mut b: Vec<&str> = b.collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
  }

  #[test]
  fn check_tables_of_newly_created_database() {
    let repo = empty_repo();

    let mut stmt = repo
      .conn
      .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
      .unwrap();
    let table_names = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();
    let table_names: Vec<_> = table_names.flatten().collect();

    unordered_eq(
      table_names.iter().map(String::as_str),
      [
        "items",
        "tag_query",
        "tag_query_data",
        "tag_query_idx",
        "tag_query_docsize",
        "tag_query_config",
      ].iter().cloned(),
    );
  }

  #[test]
  fn can_insert_items() {
    let repo = empty_repo();
    repo.insert_item("hello", "text root").unwrap();
    repo.insert_item("world", "video root").unwrap();

    let mut stmt = repo.conn.prepare("SELECT path FROM items").unwrap();
    let item_names: Vec<String> = stmt
      .query_map([], |row| row.get::<_, String>(0))
      .unwrap()
      .flatten()
      .collect();

    unordered_eq(
      item_names.iter().map(String::as_str),
      ["hello", "world"].iter().cloned(),
    );
  }

  #[test]
  fn cant_insert_duplicate_items() {
    let repo = empty_repo();

    repo.insert_item("hello", "text root").unwrap();
    let rv = repo.insert_item("hello", "video root");

    assert!(matches!(rv, Err(DatabaseError::DuplicatePathError(_))));
  }

  #[test]
  fn can_query_items() {
    fn expect_query(repo: &Repo, query: &str, expected: Vec<&str>) {
      let items = repo.get_items(Some(query)).unwrap();

      unordered_eq(
        items.iter().map(|x| x.path.as_str()),
        expected.iter().cloned(),
      );
    }

    let repo = test_repo();
    expect_query(&repo, "text", vec!["hello", "hello2", "hello4"]);
    expect_query(&repo, "video", vec!["hello3", "world"]);
    expect_query(&repo, "apple", vec!["hello4"]);
  }

  #[test]
  fn can_get_all_items() {
    let repo = test_repo();
    let items = repo.get_items(None::<String>).unwrap();
    unordered_eq(
      items.iter().map(|x| x.path.as_str()),
      ["hello", "hello2", "hello3", "hello4", "world"].iter().copied(),
    )
  }

  // #[test]
  // fn print_sqlite_version() {
  //   let repo = new_repo();
  //   let version: String = repo.conn.query_row("select sqlite_version()", [], |row| row.get(0)).unwrap();
  //   dbg!(version);
  // }
}
