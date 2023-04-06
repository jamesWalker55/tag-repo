use std::collections::{HashMap, HashSet};
use std::fs::create_dir;
use std::path::{Path, PathBuf};

use crate::diff::{diff_path_list, DiffError};
use crate::query::to_sql;
use crate::query::ParseError;
use crate::scan::{scan_dir, Options, ScanError};
use indoc::indoc;
use lazy_static::lazy_static;
use relative_path::RelativePathBuf;
use rusqlite::Error::{QueryReturnedNoRows, SqliteFailure};
use rusqlite::{ffi, params, Connection, ErrorCode, Row};
use rusqlite_migration::{Migrations, M};
use serde::Serialize;
#[cfg(test)]
use tempfile::{tempdir, TempDir};
use thiserror::Error;
use tracing::debug;

#[derive(Error, Debug)]
pub enum OpenError {
    #[error("repo path does not exist")]
    PathDoesNotExist,
    #[error("failed to create .tagrepo folder")]
    FailedToCreateRepo(#[from] std::io::Error),
    #[error("failed to create database")]
    FailedToCreateDatabase(#[from] rusqlite::Error),
    #[error("failed to migrate database")]
    FailedToMigrateDatabase(#[from] rusqlite_migration::Error),
}

#[derive(Error, Debug)]
#[deprecated]
pub enum DatabaseError {
    #[error("an error occurred in rusqlite")]
    BackendError(#[from] rusqlite::Error),
    #[error("attempted to insert path, but path already exists in database")]
    DuplicatePathError(String),
    #[error("failed to find item")]
    ItemNotFound,
}

#[derive(Error, Debug)]
pub enum InsertError {
    #[error("an error occurred in rusqlite")]
    BackendError(#[from] rusqlite::Error),
    #[error("attempted to insert path, but path already exists in database")]
    DuplicatePathError(String),
    #[error("failed to retrieve item data after inserting into database")]
    SearchError(#[from] SearchError),
}

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("an error occurred in rusqlite, {0}")]
    BackendError(#[from] rusqlite::Error),
    #[error("failed to find item")]
    ItemNotFound,
}

#[derive(Error, Debug)]
pub enum RemoveError {
    #[error("an error occurred in rusqlite, {0}")]
    BackendError(#[from] rusqlite::Error),
}

#[derive(Error, Debug)]
pub enum UpdateError {
    #[error("an error occurred in rusqlite, {0}")]
    BackendError(#[from] rusqlite::Error),
}

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("an error occurred in rusqlite, {0}")]
    BackendError(#[from] rusqlite::Error),
    #[error("invalid search query")]
    InvalidQuery,
}

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("an error occurred in rusqlite, {0}")]
    BackendError(#[from] rusqlite::Error),
    #[error("failed to diff file paths, {0}")]
    DiffError(#[from] DiffError),
    #[error("failed to retrieve all items in database, {0}")]
    SearchError(#[from] SearchError),
    #[error("failed to scan directory for a list of files, {0}")]
    ScanError(#[from] ScanError),
}

#[derive(Debug, Serialize)]
pub struct Item {
    pub(crate) id: i64,
    pub(crate) path: String,
    pub(crate) tags: String,
    pub(crate) meta_tags: String,
}

#[derive(Debug)]
pub struct Repo {
    path: PathBuf,
    conn: Connection,
}

impl Repo {
    /// Common function used to convert a query row into an item.
    ///
    /// Queried columns must be:
    ///
    /// ```sql
    /// SELECT i.id, i.path, i.tags, i.meta_tags
    /// ```
    fn to_item(row: &Row) -> Result<Item, rusqlite::Error> {
        Ok(Item {
            id: row.get::<_, i64>(0)?,
            path: row.get::<_, String>(1)?,
            tags: row.get::<_, String>(2)?,
            meta_tags: row.get::<_, String>(3)?,
        })
    }

    pub fn open(repo_path: impl AsRef<Path>) -> Result<Repo, OpenError> {
        let repo_path = repo_path.as_ref();
        if !repo_path.exists() {
            return Err(OpenError::PathDoesNotExist);
        }
        let data_path = repo_path.join(".tagrepo");
        if !data_path.exists() {
            create_dir(&data_path)?;
        }
        let db_path = data_path.join("tags.db");
        let conn = open_database(db_path)?;
        let repo = Self { path: PathBuf::from(repo_path), conn };
        Ok(repo)
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub(crate) fn insert_item<T, U>(&self, path: T, tags: U) -> Result<Item, InsertError>
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        let path = path.as_ref();
        let tags = tags.as_ref();
        let result = self.conn.execute(
            "INSERT INTO items (path, tags) VALUES (?1, ?2)",
            (&path, &tags),
        );

        match result {
            Ok(_) => {
                let id = self.conn.last_insert_rowid();
                Ok(self.get_item_by_id(id)?)
            }
            Err(SqliteFailure(
                ffi::Error { code: ErrorCode::ConstraintViolation, .. },
                Some(msg),
            )) if msg == "UNIQUE constraint failed: items.path" => {
                Err(InsertError::DuplicatePathError(path.to_string()))
            }
            Err(err) => Err(InsertError::from(err)),
        }
    }

    pub(crate) fn insert_items<T>(
        &mut self,
        items_params: impl Iterator<Item = (T, T)>,
    ) -> Result<(), InsertError>
    where
        T: AsRef<str>,
    {
        // I attempted to optimise this following this guide:
        // https://avi.im/blag/2021/fast-sqlite-inserts/

        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached("INSERT INTO items (path, tags) VALUES (?1, ?2)")?;
            for (path, tags) in items_params {
                let path = path.as_ref();
                let tags = tags.as_ref();
                stmt.execute(params![path, tags])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn get_item_by_path(&self, path: impl AsRef<str>) -> Result<Item, SearchError> {
        let path = path.as_ref();
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, tags, meta_tags FROM items WHERE path = :path LIMIT 1")?;
        let item = stmt.query_row([&path], Self::to_item);
        if let Err(QueryReturnedNoRows) = item {
            return Err(SearchError::ItemNotFound);
        }

        Ok(item?)
    }

    pub(crate) fn get_item_by_id(&self, id: i64) -> Result<Item, SearchError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, tags, meta_tags FROM items WHERE id = :id LIMIT 1")?;
        let item = stmt.query_row([id], Self::to_item);
        if let Err(QueryReturnedNoRows) = item {
            return Err(SearchError::ItemNotFound);
        }

        Ok(item?)
    }

    pub(crate) fn remove_item_by_path(&self, path: impl AsRef<str>) -> Result<(), RemoveError> {
        let path = path.as_ref();
        self.conn
            .execute("DELETE FROM items WHERE path = :path", [path])?;
        Ok(())
    }

    pub(crate) fn remove_item_by_id(&self, id: i64) -> Result<(), RemoveError> {
        self.conn
            .execute("DELETE FROM items WHERE id = :id", [id])?;
        Ok(())
    }

    pub(crate) fn update_tags(
        &self,
        item_id: i64,
        tags: impl AsRef<str>,
    ) -> Result<(), UpdateError> {
        let rv = self.conn.execute(
            "UPDATE items SET tags = :tags WHERE id = :id",
            params![tags.as_ref(), item_id],
        );
        match rv {
            Ok(_) => Ok(()),
            Err(e) => Err(UpdateError::from(e)),
        }
    }

    pub(crate) fn update_path(
        &self,
        item_id: i64,
        path: impl AsRef<str>,
    ) -> Result<(), UpdateError> {
        let path = path.as_ref();
        let rv = self.conn.execute(
            "UPDATE items SET path = :path WHERE id = :id",
            params![path, item_id],
        );
        match rv {
            Ok(_) => Ok(()),
            Err(e) => Err(UpdateError::from(e)),
        }
    }

    pub(crate) fn rename_path(
        &self,
        old_path: impl AsRef<str>,
        new_path: impl AsRef<str>,
    ) -> Result<(), UpdateError> {
        let old_path = old_path.as_ref();
        let new_path = new_path.as_ref();
        self.conn.execute(
            "UPDATE items SET path = ?2 WHERE path = ?1",
            params![old_path, new_path],
        )?;
        Ok(())
    }

    pub fn query_items<'a>(&'a self, query: &'a str) -> Result<Vec<Item>, QueryError> {
        let where_clause = to_sql(query).map_err(|x| QueryError::InvalidQuery)?;
        let sql = format!(
            indoc! {"
                SELECT i.id, i.path, i.tags, i.meta_tags
                FROM items i
                INNER JOIN
                    tag_query tq ON tq.id = i.id
                WHERE {}
            "},
            where_clause
        );
        let mut stmt = self.conn.prepare_cached(sql.as_str())?;
        let mapped_rows = stmt.query_map([], Self::to_item)?;
        let items: Result<Vec<_>, _> = mapped_rows.collect();
        Ok(items?)
    }

    pub(crate) fn all_items(&self) -> Result<Vec<Item>, SearchError> {
        let sql = "SELECT i.id, i.path, i.tags, i.meta_tags FROM items i";
        let mut stmt = self.conn.prepare_cached(sql)?;
        let mapped_rows = stmt.query_map([], Self::to_item)?;
        let items: Result<Vec<_>, _> = mapped_rows.collect();
        Ok(items?)
    }

    #[tracing::instrument(skip(new_paths))]
    pub fn sync(
        &mut self,
        new_paths: impl IntoIterator<Item = RelativePathBuf>,
    ) -> Result<(), SyncError> {
        let old_paths: HashSet<RelativePathBuf> = self
            .all_items()?
            .into_iter()
            .map(|x| RelativePathBuf::from(x.path))
            .collect();
        let new_paths: HashSet<RelativePathBuf> = new_paths.into_iter().collect();
        debug!("unique old paths: {}", old_paths.len());
        debug!("unique new paths: {}", new_paths.len());

        let path_diff = diff_path_list(&old_paths, &new_paths)?;
        debug!(
            "diff: created={}, deleted={}, renamed={}",
            path_diff.created.len(),
            path_diff.deleted.len(),
            path_diff.renamed.len(),
        );

        let tx = self.conn.transaction()?;
        {
            // delete old paths
            let mut stmt = tx.prepare_cached("DELETE FROM items WHERE path = :path")?;
            for path in &path_diff.deleted {
                stmt.execute(params![path.as_str()])?;
            }
            // create new paths
            let mut stmt = tx.prepare_cached("INSERT INTO items (path, tags) VALUES (?1, ?2)")?;
            for path in &path_diff.created {
                stmt.execute(params![path.as_str(), ""])?;
            }
            // rename existing paths
            let mut stmt = tx.prepare_cached("UPDATE items SET path = ?2 WHERE path = ?1")?;
            for (from, to) in &path_diff.renamed {
                stmt.execute(params![from.as_str(), to.as_str()])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn sync_all(&mut self) -> Result<(), SyncError> {
        self.sync(scan_dir(&self.path, Options::default()).unwrap())?;
        Ok(())
    }
}

lazy_static! {
    #[rustfmt::skip]
    static ref MIGRATIONS: Migrations<'static> =
        Migrations::new(vec![
            M::up(include_str!("migrations/01u_initial.sql"))
            .down(include_str!("migrations/01d_initial.sql")),
        ]);
}

pub(crate) fn open_database(db_path: impl AsRef<Path>) -> Result<Connection, OpenError> {
    let db_path = db_path.as_ref();
    let mut conn = Connection::open(db_path).map_err(OpenError::FailedToCreateDatabase)?;

    // https://www.sqlite.org/pragma.html
    // WAL is somehow slower. Play around with the benchmark test at the bottom of this file.
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    conn.pragma_update(None, "synchronous", "FULL").unwrap();
    conn.pragma_update(None, "locking_mode", "EXCLUSIVE")
        .unwrap();
    conn.pragma_update(None, "case_sensitive_like", false)
        .unwrap();

    MIGRATIONS
        .to_latest(&mut conn)
        .map_err(OpenError::FailedToMigrateDatabase)?;

    Ok(conn)
}

/// The only purpose of this struct is to bundle `Repo` and `TempDir` together. This ensures that
/// `TempDir` is dropped AFTER `Repo`.
///
/// Otherwise, if `TempDir` drops first, it cannot delete the temp folder as `Repo` is still using
/// the database.
#[cfg(test)]
pub(crate) struct TestRepo {
    pub(crate) repo: Repo,
    #[allow(dead_code)]
    tempdir: TempDir,
}

#[cfg(test)]
impl TestRepo {
    pub(crate) fn new() -> Self {
        let dir = tempdir().unwrap();
        let repo = Repo::open(&dir).unwrap();
        Self { repo, tempdir: dir }
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::tests::utils::assert_unordered_eq;

    use super::*;

    fn empty_testrepo() -> TestRepo {
        TestRepo::new()
    }

    /// Simple repo with 5 items in ascending alphabetical order
    fn testrepo_1() -> TestRepo {
        let tr = empty_testrepo();
        tr.repo.insert_item("apple", "food red").unwrap();
        tr.repo.insert_item("bee", "animal yellow").unwrap();
        tr.repo.insert_item("cat", "animal yellow").unwrap();
        tr.repo.insert_item("dog", "animal orange").unwrap();
        tr.repo.insert_item("egg", "food orange").unwrap();
        tr
    }

    /// Repo with all possible combinations of letters "a", "b", "c", "d", "e"
    fn testrepo_2() -> TestRepo {
        let tr = empty_testrepo();

        let possible_tags: Vec<_> = "a b c d e".split_whitespace().collect();

        let mut counter = 0;

        for i in 1..=possible_tags.len() {
            for x in possible_tags.iter().combinations(i) {
                let name = format!("item {}", counter);
                let tags = x.iter().join(" ");
                tr.repo.insert_item(name, tags).unwrap();
                counter += 1;
            }
        }

        tr
    }

    #[test]
    fn check_tables_of_newly_created_database() {
        let mut tr = empty_testrepo();
        let repo = &mut tr.repo;

        let mut stmt = repo
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let table_names = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();
        let table_names: Vec<_> = table_names.flatten().collect();

        assert_unordered_eq(
            table_names.iter().map(String::as_str),
            [
                "items",
                "tag_query",
                "tag_query_data",
                "tag_query_idx",
                "tag_query_docsize",
                "tag_query_config",
            ],
        );
    }

    #[test]
    fn can_insert_items() {
        let mut tr = empty_testrepo();
        let repo = &mut tr.repo;

        repo.insert_item("hello", "text root").unwrap();
        repo.insert_item("world", "video root").unwrap();

        let mut stmt = repo.conn.prepare("SELECT path FROM items").unwrap();
        let item_names: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .flatten()
            .collect();

        assert_unordered_eq(item_names.iter().map(String::as_str), ["hello", "world"]);
    }

    #[test]
    fn cant_insert_duplicate_items() {
        let mut tr = empty_testrepo();
        let repo = &mut tr.repo;

        repo.insert_item("hello", "text root").unwrap();
        let rv = repo.insert_item("hello", "video root");

        assert!(matches!(rv, Err(InsertError::DuplicatePathError(_))));
    }

    #[test]
    fn can_query_items() {
        fn expect_query(repo: &Repo, query: &str, expected: Vec<&str>) {
            let items = repo.query_items(query).unwrap();

            assert_unordered_eq(items.iter().map(|x| x.path.as_str()), expected);
        }

        let mut tr = testrepo_1();
        let repo = &mut tr.repo;

        expect_query(&repo, "animal", vec!["bee", "cat", "dog"]);
        expect_query(&repo, "food", vec!["apple", "egg"]);
        expect_query(&repo, "yellow", vec!["bee", "cat"]);
    }

    #[test]
    fn can_get_all_items() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;
        let items = repo.query_items("").unwrap();
        assert_unordered_eq(
            items.iter().map(|x| x.path.as_str()),
            ["apple", "bee", "cat", "dog", "egg"],
        )
    }

    #[test]
    fn can_get_item_by_path() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;
        let item = repo.get_item_by_path("apple").unwrap();
        assert_eq!(item.id, 1);
        assert_eq!(item.path, "apple");
        assert_eq!(item.tags, "food red");
    }

    #[test]
    fn can_get_item_by_id() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;
        let item = repo.get_item_by_id(1).unwrap();
        assert_eq!(item.id, 1);
        assert_eq!(item.path, "apple");
        assert_eq!(item.tags, "food red");
    }

    #[test]
    fn can_remove_item_by_path() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;
        repo.remove_item_by_path("apple").unwrap();
        let rv = repo.get_item_by_path("apple");
        assert!(matches!(rv, Err(SearchError::ItemNotFound)))
    }

    #[test]
    fn can_remove_item_by_id() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;
        repo.remove_item_by_id(1).unwrap();
        let rv = repo.get_item_by_id(1);
        assert!(matches!(rv, Err(SearchError::ItemNotFound)))
    }

    #[test]
    fn can_update_item_tags() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;

        let item = repo.get_item_by_path("apple").unwrap();
        let new_tags = "computer laptop";
        repo.update_tags(item.id, new_tags).unwrap();

        // fetch item again
        let item = repo.get_item_by_path("apple").unwrap();
        assert_eq!(item.tags, new_tags);
    }

    #[test]
    fn can_update_item_path() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;

        let item = repo.get_item_by_id(1).unwrap();
        let new_path = "pizza";
        repo.update_path(item.id, new_path).unwrap();

        // fetch item again
        let item = repo.get_item_by_id(1).unwrap();
        assert_eq!(item.path, new_path);
    }

    #[test]
    /// not really a test, just some code to manually test queries
    fn query_test() {
        let tr = testrepo_2();

        // The query:
        //
        //     a b -e in:1 | d e in:0
        //
        let sql = indoc! {r#"
            SELECT i.path, i.tags
            FROM items i
            WHERE
                i.id IN ( SELECT id FROM tag_query('tags:"a" tags:"b" AND ("meta_tags": "all") NOT tags:"e"') )
                AND i.path LIKE '%1%'
            OR
                i.id IN ( SELECT id FROM tag_query('tags:"d" tags:"e"') )
                AND i.path LIKE '%0%'
        "#};

        let conn = tr.repo.conn;

        let mut stmt = conn.prepare(&sql).unwrap();
        let out = stmt
            .query_map([], |row| {
                let path = row.get::<_, String>(0)?;
                let tags = row.get::<_, String>(1)?;
                let out = format!("{: >8}: {}", path, tags);

                Ok(out)
            })
            .unwrap();

        let mut count = 0;
        for x in out {
            println!("{}", x.unwrap());
            count += 1;
        }
        println!("Got {} rows.", count);

        ()
    }

    #[test]
    fn query_test_2() {
        let tr = testrepo_2();
        let items = tr.repo.query_items("a b -c").unwrap();
        dbg!(items);
    }

    // #[test]
    // fn print_sqlite_version() {
    //   let repo = new_repo();
    //   let version: String = repo.conn.query_row("select sqlite_version()", [], |row| row.get(0)).unwrap();
    //   dbg!(version);
    // }

    mod scan_integration {
        use std::time::Instant;

        use crate::scan::scan_dir;

        use super::*;

        #[test]
        fn my_test() {
            println!("Creating repo");
            let start = Instant::now();
            let mut tr = empty_testrepo();
            let repo = &mut tr.repo;
            println!("  Took: {:?}", start.elapsed());

            println!("Scanning dir");
            let start = Instant::now();
            let paths = scan_dir(r#"D:\Audio Samples\"#, Options::default()).unwrap();
            println!("  Took: {:?}", start.elapsed());

            println!("Adding paths");
            println!("  Inserting {} paths...", paths.len());
            let start = Instant::now();
            repo.insert_items(paths.iter().map(|p| (p.as_str(), "asd")))
                .unwrap();
            println!("  Took: {:?}", start.elapsed());
            println!("Done!");
        }
    }
}
