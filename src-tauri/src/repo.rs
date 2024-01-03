use std::collections::HashSet;
use std::fs::create_dir;
use std::path::{Path, PathBuf};

use indoc::indoc;
use itertools::Itertools;
use lazy_static::lazy_static;
use relative_path::RelativePathBuf;
use rusqlite::functions::FunctionFlags;
use rusqlite::Error::{QueryReturnedNoRows, SqliteFailure};
use rusqlite::{ffi, params, Connection, ErrorCode, Row};
use rusqlite_migration::{Migrations, M};
use serde::Serialize;
#[cfg(test)]
use tempfile::{tempdir, TempDir};
use thiserror::Error;
use tracing::debug;

use crate::diff::{diff_path_list, DiffError};
use crate::query::to_sql;
use crate::scan::{scan_dir, Options, ScanError};
use crate::tree::{from_ordered_paths, FolderBuf, PathTreeError};

type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;

#[derive(Error, Debug)]
pub enum OpenError {
    #[error("repo path does not exist")]
    PathDoesNotExist,
    #[error("failed to create .tagrepo folder")]
    FailedToCreateRepo(#[from] std::io::Error),
    #[error("failed to create database")]
    FailedToOpenDatabase(#[from] r2d2::Error),
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
    #[error("failed to fetch item, {0}")]
    SearchError(#[from] SearchError),
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

#[derive(Error, Debug)]
pub enum InsertTagsError {
    #[error("an error occurred in rusqlite, {0}")]
    BackendError(#[from] rusqlite::Error),
}

#[derive(Error, Debug)]
pub enum RemoveTagsError {
    #[error("an error occurred in rusqlite, {0}")]
    BackendError(#[from] rusqlite::Error),
}

#[derive(Error, Debug)]
pub enum DirStructureError {
    #[error("an error occurred in rusqlite, {0}")]
    BackendError(#[from] rusqlite::Error),
    #[error("malformed path, {0}")]
    MalformedPath(PathBuf),
}

#[derive(Debug, Serialize, Clone)]
pub struct Item {
    pub(crate) id: i64,
    pub(crate) path: String,
    pub(crate) tags: Vec<String>,
    pub(crate) meta_tags: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Tag {
    pub(crate) name: String,
    pub(crate) column: String,
    pub(crate) count: i64,
}

impl Tag {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            name: row.get::<_, String>("term")?,
            column: row.get::<_, String>("col")?,
            count: row.get::<_, i64>("cnt")?,
        })
    }
}

#[derive(Debug)]
pub struct Repo {
    path: PathBuf,
    pool: Pool,
}

fn repeat_vars(count: usize) -> String {
    assert_ne!(count, 0);
    let mut s = "?,".repeat(count);
    // Remove trailing comma
    s.pop();
    s
}

pub trait IntoTags {
    fn into_tags(self) -> Vec<String>;
}

impl IntoTags for String {
    fn into_tags(self) -> Vec<String> {
        self.split_whitespace()
            .map(|x| x.to_string())
            .sorted()
            .collect()
    }
}

impl IntoTags for &str {
    fn into_tags(self) -> Vec<String> {
        self.split_whitespace()
            .map(|x| x.to_string())
            .sorted()
            .collect()
    }
}

impl IntoTags for Vec<String> {
    fn into_tags(self) -> Vec<String> {
        self.iter().cloned().sorted().collect()
    }
}

impl IntoTags for &Vec<String> {
    fn into_tags(self) -> Vec<String> {
        self.iter().cloned().sorted().collect()
    }
}

impl IntoTags for Vec<&str> {
    fn into_tags(self) -> Vec<String> {
        self.iter().map(|x| x.to_string()).sorted().collect()
    }
}

impl IntoTags for &Vec<&str> {
    fn into_tags(self) -> Vec<String> {
        self.iter().map(|x| x.to_string()).sorted().collect()
    }
}

impl Repo {
    /// Common function used to convert a query row into an item.
    ///
    /// Queried columns must be:
    ///
    /// ```sql
    /// SELECT i.id, i.path, i.tags, i.meta_tags
    /// ```
    fn row_to_item(row: &Row) -> Result<Item, rusqlite::Error> {
        Ok(Item {
            id: row.get::<_, i64>(0)?,
            path: row.get::<_, String>(1)?,
            tags: Self::convert_raw_tags(row.get::<_, String>(2)?),
            meta_tags: row.get::<_, String>(3)?,
        })
    }

    /// Common function used to convert a query row into a id.
    ///
    /// Queried columns must be:
    ///
    /// ```sql
    /// SELECT i.id
    /// ```
    fn row_to_id(row: &Row) -> Result<i64, rusqlite::Error> {
        row.get::<_, i64>(0)
    }

    /// Convert a raw tag string from the database into a vector of strings
    fn convert_raw_tags(raw_tags: String) -> Vec<String> {
        if raw_tags.is_empty() {
            // we MUST handle the empty case separately, because if you call #split() on an empty
            // string, you get a single element ""
            vec![]
        } else {
            raw_tags.split(" ").map(String::from).collect()
        }
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
        let pool = open_database(db_path)?;
        let repo = Self { path: PathBuf::from(repo_path), pool };
        Ok(repo)
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub(crate) fn insert_item<T, U>(&self, path: T, tags: U) -> Result<Item, InsertError>
    where
        T: AsRef<str>,
        U: IntoTags,
    {
        let path = path.as_ref();
        let tags = tags.into_tags();
        let conn = self.pool.get().unwrap();
        let result = conn.execute(
            "INSERT INTO items (path, tags) VALUES (?1, ?2)",
            (&path, tags.join(" ")),
        );

        match result {
            Ok(_) => {
                let id = conn.last_insert_rowid();
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

    pub(crate) fn insert_items<T, U>(
        &self,
        items_params: impl Iterator<Item = (T, U)>,
    ) -> Result<(), InsertError>
    where
        T: AsRef<str>,
        U: IntoTags,
    {
        // I attempted to optimise this following this guide:
        // https://avi.im/blag/2021/fast-sqlite-inserts/

        let mut conn = self.pool.get().unwrap();
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached("INSERT INTO items (path, tags) VALUES (?1, ?2)")?;
            for (path, tags) in items_params {
                let path = path.as_ref();
                let tags = tags.into_tags();
                stmt.execute(params![path, tags.join(" ")])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn get_item_by_path(&self, path: impl AsRef<str>) -> Result<Item, SearchError> {
        let path = path.as_ref();
        let conn = self.pool.get().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, path, tags, meta_tags FROM items WHERE path = :path LIMIT 1")?;
        let item = stmt.query_row([&path], Self::row_to_item);
        if let Err(QueryReturnedNoRows) = item {
            return Err(SearchError::ItemNotFound);
        }

        Ok(item?)
    }

    pub(crate) fn get_item_by_id(&self, id: i64) -> Result<Item, SearchError> {
        let conn = self.pool.get().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, path, tags, meta_tags FROM items WHERE id = :id LIMIT 1")?;
        let item = stmt.query_row([id], Self::row_to_item);
        if let Err(QueryReturnedNoRows) = item {
            return Err(SearchError::ItemNotFound);
        }

        Ok(item?)
    }

    pub(crate) fn remove_item_by_path(&self, path: impl AsRef<str>) -> Result<Item, RemoveError> {
        let removed_item = self.get_item_by_path(&path)?;
        let path = path.as_ref();
        let conn = self.pool.get().unwrap();
        conn.execute("DELETE FROM items WHERE path = :path", [path])?;
        Ok(removed_item)
    }

    pub(crate) fn remove_item_by_id(&self, id: i64) -> Result<(), RemoveError> {
        let conn = self.pool.get().unwrap();
        conn.execute("DELETE FROM items WHERE id = :id", [id])?;
        Ok(())
    }

    pub(crate) fn update_tags(&self, item_id: i64, tags: impl IntoTags) -> Result<(), UpdateError> {
        let conn = self.pool.get().unwrap();
        let rv = conn.execute(
            "UPDATE items SET tags = :tags WHERE id = :id",
            params![tags.into_tags().join(" "), item_id],
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
        let conn = self.pool.get().unwrap();
        let rv = conn.execute(
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
        let conn = self.pool.get().unwrap();
        conn.execute(
            "UPDATE items SET path = ?2 WHERE path = ?1",
            params![old_path, new_path],
        )?;
        Ok(())
    }

    pub(crate) fn insert_tags(
        &self,
        item_id: i64,
        tags: impl IntoTags,
    ) -> Result<(), InsertTagsError> {
        let tags = tags.into_tags();
        if tags.len() == 0 {
            return Ok(());
        }
        let sql = format!(
            "UPDATE items SET tags = insert_tags(tags, {}) WHERE id = ?",
            // this function will panic if you give it 0 length
            repeat_vars(tags.len()),
        );
        // converting item_id to a string is fine, sqlite converts types dynamically
        let item_id = item_id.to_string();
        let conn = self.pool.get().unwrap();
        conn.execute(
            &sql,
            rusqlite::params_from_iter(tags.iter().chain(Some(&item_id))),
        )?;
        Ok(())
    }

    pub(crate) fn batch_insert_tags(
        &self,
        item_ids: &Vec<i64>,
        tags: impl IntoTags,
    ) -> Result<(), InsertTagsError> {
        if item_ids.len() == 0 {
            return Ok(());
        }
        let tags = tags.into_tags();
        if tags.len() == 0 {
            return Ok(());
        }

        let sql = format!(
            "UPDATE items SET tags = insert_tags(tags, {}) WHERE id IN ({})",
            // this function will panic if you give it 0 length
            repeat_vars(tags.len()),
            repeat_vars(item_ids.len()),
        );
        let item_ids: Vec<_> = item_ids.iter().map(|x| x.to_string()).collect();
        let conn = self.pool.get().unwrap();
        conn.execute(
            &sql,
            // converting item_id to a string is fine, sqlite converts types dynamically
            rusqlite::params_from_iter(tags.iter().chain(item_ids.iter())),
        )?;
        Ok(())
    }

    pub(crate) fn remove_tags(
        &self,
        item_id: i64,
        tags: impl IntoTags,
    ) -> Result<(), RemoveTagsError> {
        let tags = tags.into_tags();
        if tags.len() == 0 {
            return Ok(());
        }
        let sql = format!(
            "UPDATE items SET tags = remove_tags(tags, {}) WHERE id = ?",
            // this function will panic if you give it 0 length
            repeat_vars(tags.len()),
        );
        // converting item_id to a string is fine, sqlite converts types dynamically
        let item_id = item_id.to_string();
        let conn = self.pool.get().unwrap();
        conn.execute(
            &sql,
            rusqlite::params_from_iter(tags.iter().chain(Some(&item_id))),
        )?;
        Ok(())
    }

    pub(crate) fn batch_remove_tags(
        &self,
        item_ids: &Vec<i64>,
        tags: impl IntoTags,
    ) -> Result<(), RemoveTagsError> {
        if item_ids.len() == 0 {
            return Ok(());
        }
        let tags = tags.into_tags();
        if tags.len() == 0 {
            return Ok(());
        }

        let sql = format!(
            "UPDATE items SET tags = remove_tags(tags, {}) WHERE id IN ({})",
            // this function will panic if you give it 0 length
            repeat_vars(tags.len()),
            repeat_vars(item_ids.len()),
        );
        let item_ids: Vec<_> = item_ids.iter().map(|x| x.to_string()).collect();
        let conn = self.pool.get().unwrap();
        conn.execute(
            &sql,
            // converting item_id to a string is fine, sqlite converts types dynamically
            rusqlite::params_from_iter(tags.iter().chain(item_ids.iter())),
        )?;
        Ok(())
    }

    pub fn query_items<'a>(&'a self, query: &'a str) -> Result<Vec<Item>, QueryError> {
        let where_clause = to_sql(query).map_err(|_x| QueryError::InvalidQuery)?;
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
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare_cached(sql.as_str())?;
        let mapped_rows = stmt.query_map([], Self::row_to_item)?;
        let items: Result<Vec<_>, _> = mapped_rows.collect();
        Ok(items?)
    }

    pub fn query_ids<'a>(&'a self, query: &'a str) -> Result<Vec<i64>, QueryError> {
        let where_clause = to_sql(query).map_err(|_x| QueryError::InvalidQuery)?;
        let sql = format!(
            indoc! {"
                SELECT i.id
                FROM items i
                INNER JOIN
                    tag_query tq ON tq.id = i.id
                WHERE {}
                ORDER BY i.path
            "},
            where_clause
        );
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare_cached(sql.as_str())?;
        let mapped_rows = stmt.query_map([], Self::row_to_id)?;
        let items: Result<Vec<_>, _> = mapped_rows.collect();
        Ok(items?)
    }

    pub fn tags(&self) -> Result<Vec<Tag>, QueryError> {
        let sql = indoc! {"
            SELECT t.term, t.col, t.doc, t.cnt
            FROM tags_col t
            ORDER BY doc DESC
        "};
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare_cached(sql)?;
        let mapped_rows = stmt.query_map([], Tag::from_row)?;
        let items: Result<Vec<_>, _> = mapped_rows.collect();
        Ok(items?)
    }

    pub(crate) fn all_items(&self) -> Result<Vec<Item>, rusqlite::Error> {
        let sql = "SELECT i.id, i.path, i.tags, i.meta_tags FROM items i";
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare_cached(sql)?;
        let mapped_rows = stmt.query_map([], Self::row_to_item)?;
        let items: Result<Vec<_>, _> = mapped_rows.collect();
        Ok(items?)
    }

    pub fn all_folders(&self) -> Result<Vec<String>, rusqlite::Error> {
        let sql = "SELECT DISTINCT dirname(i.path) FROM items i ORDER BY dirname(i.path)";
        let conn = self.pool.get().unwrap();
        let mut stmt = conn.prepare_cached(sql)?;
        let mapped_rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let items: Result<Vec<_>, _> = mapped_rows.collect();
        Ok(items?)
    }

    pub fn dir_structure(&self) -> Result<FolderBuf, DirStructureError> {
        let paths = self.all_folders()?;
        let dirs = from_ordered_paths(&paths).map_err(|x| match x {
            PathTreeError::MalformedPath(path) => DirStructureError::MalformedPath(path),
        })?;
        Ok(dirs)
    }

    #[tracing::instrument(skip(new_paths))]
    pub fn sync(
        &self,
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

        let mut conn = self.pool.get().unwrap();
        let tx = conn.transaction()?;
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

    pub fn sync_all(&self) -> Result<(), SyncError> {
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

fn add_functions(conn: &Connection) -> rusqlite::Result<()> {
    conn.create_scalar_function(
        "validate_tags",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            assert_eq!(ctx.len(), 1, "called with unexpected number of arguments");

            let input = ctx.get::<String>(0)?;
            let result: String = input.split_ascii_whitespace().sorted().join(" ");
            Ok(result)
        },
    )?;
    conn.create_scalar_function(
        "insert_tags",
        -1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            assert!(ctx.len() >= 2, "at least 2 arguments must be given");

            let old_tags = ctx.get::<String>(0)?;
            let mut old_tags = old_tags.into_tags();

            for i in 1..ctx.len() {
                let new_tag = ctx.get::<String>(i)?;
                if new_tag.is_empty() {
                    continue;
                }
                match old_tags.binary_search(&new_tag) {
                    Ok(_pos) => { /* already in list, do nothing */ }
                    Err(pos) => old_tags.insert(pos, new_tag),
                }
            }
            Ok(old_tags.join(" "))
        },
    )?;
    conn.create_scalar_function(
        "remove_tags",
        -1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            assert!(ctx.len() >= 2, "at least 2 arguments must be given");

            let old_tags = ctx.get::<String>(0)?;
            let mut old_tags = old_tags.into_tags();

            for i in 1..ctx.len() {
                let tag_to_remove = ctx.get::<String>(i)?;
                if tag_to_remove.is_empty() {
                    continue;
                }
                match old_tags.binary_search(&tag_to_remove) {
                    Ok(pos) => {
                        old_tags.remove(pos);
                    }
                    Err(_pos) => { /* not in list, do nothing */ }
                }
            }
            Ok(old_tags.join(" "))
        },
    )?;
    conn.create_scalar_function(
        "dirname",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            assert_eq!(ctx.len(), 1, "called with unexpected number of arguments");

            let fullpath = ctx.get::<String>(0)?;
            let fullpath: &Path = fullpath.as_ref();
            let parent = fullpath.parent().unwrap();

            Ok(parent.to_str().unwrap().to_string())
        },
    )?;
    conn.create_scalar_function(
        "extname",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            assert_eq!(ctx.len(), 1, "called with unexpected number of arguments");

            let fullpath = ctx.get::<String>(0)?;
            let fullpath: &Path = fullpath.as_ref();
            match fullpath.extension() {
                None => Ok(String::from("")),
                Some(extension) => Ok(extension.to_str().unwrap().to_string()),
            }
        },
    )?;
    Ok(())
}

pub(crate) fn open_database(db_path: impl AsRef<Path>) -> Result<Pool, OpenError> {
    let db_path = db_path.as_ref();
    let manager = r2d2_sqlite::SqliteConnectionManager::file(db_path).with_init(|conn| {
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.pragma_update(None, "synchronous", "FULL").unwrap();
        conn.pragma_update(None, "locking_mode", "EXCLUSIVE")
            .unwrap();
        conn.pragma_update(None, "case_sensitive_like", false)
            .unwrap();

        add_functions(&conn).unwrap();

        Ok(())
    });
    let pool = r2d2::Pool::new(manager).map_err(OpenError::FailedToOpenDatabase)?;

    {
        let mut conn = pool.get().map_err(OpenError::FailedToOpenDatabase)?;

        // https://www.sqlite.org/pragma.html
        // WAL is somehow slower. Play around with the benchmark test at the bottom of this file.
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();

        // migrate to latest version
        MIGRATIONS
            .to_latest(&mut conn)
            .map_err(OpenError::FailedToMigrateDatabase)?;
    }

    Ok(pool)
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

        let conn = repo.pool.get().unwrap();
        let mut stmt = conn
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
                "tags_col",
            ],
        );
    }

    #[test]
    fn can_insert_items() {
        let mut tr = empty_testrepo();
        let repo = &mut tr.repo;

        repo.insert_item("hello", "text root").unwrap();
        repo.insert_item("world", "video root").unwrap();

        let conn = repo.pool.get().unwrap();
        let mut stmt = conn.prepare("SELECT path FROM items").unwrap();
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
        assert_eq!(item.tags, vec!["food", "red"]);
    }

    #[test]
    fn can_get_item_by_id() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;
        let item = repo.get_item_by_id(1).unwrap();
        assert_eq!(item.id, 1);
        assert_eq!(item.path, "apple");
        assert_eq!(item.tags, vec!["food", "red"]);
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
        let new_tags: Vec<_> = new_tags.split(" ").map(String::from).collect();
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

        let conn = tr.repo.pool.get().unwrap();

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

    #[test]
    fn custom_validate_tags_1() {
        let tr = empty_testrepo();
        let input = "a b c";
        let expected = "a b c";
        let result: String = tr
            .repo
            .pool
            .get()
            .unwrap()
            .query_row("SELECT validate_tags(?1)", params![input], |row| row.get(0))
            .unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn custom_validate_tags_2() {
        let tr = empty_testrepo();
        let input = "  c  b  a  ";
        let expected = "a b c";
        let result: String = tr
            .repo
            .pool
            .get()
            .unwrap()
            .query_row("SELECT validate_tags(?1)", params![input], |row| row.get(0))
            .unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn custom_validate_tags_3() {
        let tr = empty_testrepo();
        let input = "   ";
        let expected = "";
        let result: String = tr
            .repo
            .pool
            .get()
            .unwrap()
            .query_row("SELECT validate_tags(?1)", params![input], |row| row.get(0))
            .unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn custom_insert_tags_1() {
        let tr = empty_testrepo();
        let result: String = tr
            .repo
            .pool
            .get()
            .unwrap()
            .query_row(
                "SELECT insert_tags(?, ?, ?, ?, ?)",
                params!["", "b", "a", "d", "asdq"],
                |row| row.get(0),
            )
            .unwrap();
        let expected = "a asdq b d";
        assert_eq!(result, expected);
    }

    #[test]
    fn custom_insert_tags_2() {
        let tr = empty_testrepo();
        let result: String = tr
            .repo
            .pool
            .get()
            .unwrap()
            .query_row(
                "SELECT insert_tags(?, ?, ?, ?, ?, ?, ?)",
                params!["bee egg", "apple", "bee", "banana", "cat", "", "fish"],
                |row| row.get(0),
            )
            .unwrap();
        let expected = "apple banana bee cat egg fish";
        assert_eq!(result, expected);
    }

    #[test]
    fn custom_remove_tags_1() {
        let tr = empty_testrepo();
        let result: String = tr
            .repo
            .pool
            .get()
            .unwrap()
            .query_row(
                "SELECT remove_tags(?, ?, ?)",
                params!["a asdq b d fish goat", "asdq", "d"],
                |row| row.get(0),
            )
            .unwrap();
        let expected = "a b fish goat";
        assert_eq!(result, expected);
    }

    #[test]
    fn custom_remove_tags_2() {
        let tr = empty_testrepo();
        let result: String = tr
            .repo
            .pool
            .get()
            .unwrap()
            .query_row(
                "SELECT remove_tags(?, ?, ?, ?, ?)",
                params![
                    "apple banana bee cat egg fish",
                    "cat",
                    "yqwfeuwqbfduq",
                    "apple",
                    "fish"
                ],
                |row| row.get(0),
            )
            .unwrap();
        let expected = "banana bee egg";
        assert_eq!(result, expected);
    }

    #[test]
    fn can_insert_tags_1() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;

        // id 1 must be "apple"
        let item = repo.get_item_by_id(1).unwrap();
        let old_tags: Vec<_> = vec!["food", "red"].into_iter().map(String::from).collect();
        assert_eq!(item.path, "apple");
        assert_eq!(item.tags, old_tags);

        // insert some tags to it
        let inserted_tags = vec!["fruit", "plant"];
        repo.insert_tags(item.id, &inserted_tags).unwrap();

        // check that the tags have been added
        let item = repo.get_item_by_id(1).unwrap();
        let new_tags: Vec<_> = vec!["food", "fruit", "plant", "red"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(item.tags, new_tags);
    }

    #[test]
    fn can_batch_insert_tags_1() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;

        // id 1 must be "apple"
        let item = repo.get_item_by_id(1).unwrap();
        let old_tags: Vec<_> = vec!["food", "red"].into_iter().map(String::from).collect();
        assert_eq!(item.path, "apple");
        assert_eq!(item.tags, old_tags);
        // id 2 must be "bee"
        let item = repo.get_item_by_id(2).unwrap();
        let old_tags: Vec<_> = vec!["animal", "yellow"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(item.path, "bee");
        assert_eq!(item.tags, old_tags);

        // insert some tags to it
        let inserted_tags = vec!["aaaa", "bbbb", "ffff", "zzzz", ""];
        repo.batch_insert_tags(&vec![1i64, 2i64], &inserted_tags)
            .unwrap();

        // check that the tags have been added
        let item = repo.get_item_by_id(1).unwrap();
        let new_tags: Vec<_> = vec!["aaaa", "bbbb", "ffff", "food", "red", "zzzz"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(item.tags, new_tags);

        let item = repo.get_item_by_id(2).unwrap();
        let new_tags: Vec<_> = vec!["aaaa", "animal", "bbbb", "ffff", "yellow", "zzzz"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(item.tags, new_tags);
    }

    #[test]
    fn can_batch_insert_tags_then_query() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;

        // insert some tags to items 1, 2
        let inserted_tags = vec!["aaaa", "bbbb", "ffff", "zzzz", ""];
        repo.batch_insert_tags(&vec![1i64, 2i64], &inserted_tags)
            .unwrap();

        // check that the tags have been added
        let item = repo.get_item_by_id(1).unwrap();
        let new_tags: Vec<_> = vec!["aaaa", "bbbb", "ffff", "food", "red", "zzzz"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(item.tags, new_tags);

        let item = repo.get_item_by_id(2).unwrap();
        let new_tags: Vec<_> = vec!["aaaa", "animal", "bbbb", "ffff", "yellow", "zzzz"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(item.tags, new_tags);

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
    fn can_remove_tags_1() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;

        // id 1 must be "apple"
        let item = repo.get_item_by_id(1).unwrap();
        let old_tags: Vec<_> = vec!["food", "red"].into_iter().map(String::from).collect();
        assert_eq!(item.path, "apple");
        assert_eq!(item.tags, old_tags);

        // remove some tags to it
        let removed_tags = vec!["food"];
        repo.remove_tags(item.id, &removed_tags).unwrap();

        // check that the tags have been added
        let item = repo.get_item_by_id(1).unwrap();
        let new_tags: Vec<_> = vec!["red"].into_iter().map(String::from).collect();
        assert_eq!(item.tags, new_tags);
    }

    #[test]
    fn can_batch_remove_tags_1() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;

        // id 1 must be "apple"
        let item = repo.get_item_by_id(1).unwrap();
        let old_tags: Vec<_> = vec!["food", "red"].into_iter().map(String::from).collect();
        assert_eq!(item.path, "apple");
        assert_eq!(item.tags, old_tags);
        // id 2 must be "bee"
        let item = repo.get_item_by_id(2).unwrap();
        let old_tags: Vec<_> = vec!["animal", "yellow"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(item.path, "bee");
        assert_eq!(item.tags, old_tags);

        // remove some tags to it
        let removed_tags = vec!["food", "animal", "qwerty", "yellow", ""];
        repo.batch_remove_tags(&vec![1i64, 2i64], &removed_tags)
            .unwrap();

        // check that the tags have been added
        let item = repo.get_item_by_id(1).unwrap();
        let new_tags: Vec<_> = vec!["red"].into_iter().map(String::from).collect();
        assert_eq!(item.tags, new_tags);

        let item = repo.get_item_by_id(2).unwrap();
        let new_tags: Vec<String> = vec![];
        assert_eq!(item.tags, new_tags);
    }

    // #[test]
    // fn print_sqlite_version() {
    //   let repo = new_repo();
    //   let version: String = repo.conn.query_row("select sqlite_version()", [], |row| row.get(0)).unwrap();
    //   dbg!(version);
    // }

    mod scan_integration {

        // #[test]
        // fn my_test() {
        //     println!("Creating repo");
        //     let start = Instant::now();
        //     let mut tr = empty_testrepo();
        //     let repo = &mut tr.repo;
        //     println!("  Took: {:?}", start.elapsed());
        //
        //     println!("Scanning dir");
        //     let start = Instant::now();
        //     let paths = scan_dir(r#"D:\Audio Samples\"#, Options::default()).unwrap();
        //     println!("  Took: {:?}", start.elapsed());
        //
        //     println!("Adding paths");
        //     println!("  Inserting {} paths...", paths.len());
        //     let start = Instant::now();
        //     repo.insert_items(paths.iter().map(|p| (p.as_str(), "asd")))
        //         .unwrap();
        //     println!("  Took: {:?}", start.elapsed());
        //     println!("Done!");
        // }
    }
}
