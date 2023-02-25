use std::fs::create_dir;
use std::path::{Path, PathBuf};
use rusqlite::{Connection, ErrorCode, Result};
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

pub struct Repo {
  conn: Connection,
}

impl Repo {
  fn insert_item<T>(&self, path: T, tags: T) -> Result<(), DatabaseError>
    where T: AsRef<str> {
    let path = path.as_ref();
    let tags = tags.as_ref();
    let result = self.conn.execute(
      "INSERT INTO items (path, tags) VALUES (?1, ?2)",
      (path, tags),
    );

    match result {
      Ok(_) => Ok(()),
      Err(SqliteFailure(sqlite_err, Some(msg))) => {
        if sqlite_err.code == ErrorCode::ConstraintViolation
          && msg == "UNIQUE constraint failed: items.path" {
          Err(DatabaseError::DuplicatePathError(path.into()))
        } else {
          Err(DatabaseError::BackendError(SqliteFailure(sqlite_err, Some(msg))))
        }
      }
      Err(err) => Err(DatabaseError::BackendError(err))
    }
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
  MIGRATIONS.to_latest(&mut conn).map_err(OpenError::FailedToMigrateDatabase)?;
  Ok(conn)
}

pub fn open(repo_path: impl AsRef<Path>) -> Result<Repo, OpenError> {
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
  let repo = Repo { conn };
  Ok(repo)
}

#[cfg(test)]
mod tests {
  use std::collections::HashSet;
  use super::*;
  use tempfile::tempdir;

  fn new_repo() -> Repo {
    open(tempdir().unwrap()).unwrap()
  }

  #[test]
  fn check_tables_of_newly_created_database() {
    let repo = new_repo();

    let mut stmt = repo.conn
      .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").unwrap();
    let table_names = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();
    let table_names: Vec<_> = table_names.flatten().collect();

    let expected_names = [
      "items",
      "tag_query",
      "tag_query_data",
      "tag_query_idx",
      "tag_query_docsize",
      "tag_query_config",
    ];

    let a: HashSet<_> = table_names.iter().map(|x| x.to_string()).collect();
    let b: HashSet<_> = expected_names.iter().map(|x| x.to_string()).collect();
    assert_eq!(a, b);
  }

  #[test]
  fn can_insert_items() {
    let repo = new_repo();
    repo.insert_item("hello", "text root").unwrap();
    repo.insert_item("world", "video root").unwrap();

    let mut stmt = repo.conn.prepare("SELECT path FROM items").unwrap();
    let item_names = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();

    let expected_names = [
      "hello",
      "world",
    ];
    for (name, expected_name) in item_names.zip(expected_names) {
      assert_eq!(name.unwrap(), expected_name);
    }

    ()
  }

  #[test]
  fn cant_insert_duplicate_items() {
    let repo = new_repo();

    repo.insert_item("hello", "text root").unwrap();
    let rv = repo.insert_item("hello", "video root");

    assert!(matches!(rv, Err(DatabaseError::DuplicatePathError(_))));
  }

  #[test]
  fn print_sqlite_version() {
    let repo = new_repo();
    let version: String = repo.conn.query_row("select sqlite_version()", [], |row| row.get(0)).unwrap();
    dbg!(version);
  }
}
