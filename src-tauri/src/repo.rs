use std::fs::create_dir;
use std::path::{Path, PathBuf};

use indoc::indoc;
use lazy_static::lazy_static;
use rusqlite::Error::{QueryReturnedNoRows, SqliteFailure};
use rusqlite::{params, Connection, ErrorCode, Row};
use rusqlite_migration::{Migrations, M};

#[derive(Debug)]
pub enum OpenError {
    PathDoesNotExist,
    FailedToCreateRepo(std::io::Error),
    FailedToCreateDatabase(rusqlite::Error),
    FailedToMigrateDatabase(rusqlite_migration::Error),
}

#[derive(Debug)]
pub enum DatabaseError {
    DuplicatePathError(String),
    ItemNotFound,
    BackendError(rusqlite::Error),
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(error: rusqlite::Error) -> Self {
        DatabaseError::BackendError(error)
    }
}

#[derive(Debug)]
pub struct Item {
    id: i64,
    path: String,
    tags: String,
    meta_tags: String,
}

pub struct Repo {
    path: PathBuf,
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
        let conn = open_database(db_path)?;
        let repo = Self {
            path: PathBuf::from(repo_path),
            conn,
        };
        Ok(repo)
    }

    fn insert_item<T>(&self, path: T, tags: T) -> Result<Item, DatabaseError>
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
            Ok(_) => {
                let id = self.conn.last_insert_rowid();
                self.get_item_by_id(id)
            }
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

    fn insert_items<T>(
        &mut self,
        items_params: impl Iterator<Item = (T, T)>,
    ) -> Result<(), DatabaseError>
    where
        T: AsRef<str>,
    {
        // I attempted to optimise this following this guide:
        // https://avi.im/blag/2021/fast-sqlite-inserts/

        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached("INSERT INTO ITEMS (path, tags) VALUES (?1, ?2)")?;
            for (path, tags) in items_params {
                let path = path.as_ref();
                let tags = tags.as_ref();
                stmt.execute(params![path, tags])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    fn get_items(&self, query: Option<&str>) -> Result<Vec<Item>, DatabaseError> {
        let to_item_closure: fn(&Row) -> Result<Item, rusqlite::Error> = |row: &Row| {
            Ok(Item {
                id: row.get::<_, i64>(0)?,
                path: row.get::<_, String>(1)?,
                tags: row.get::<_, String>(2)?,
                meta_tags: row.get::<_, String>(3)?,
            })
        };

        let mut stmt;

        let mapped_rows = match query {
            Some(query) => {
                stmt = self.conn.prepare(indoc! {"
                    SELECT i.id, i.path, i.tags, i.meta_tags
                    FROM items i
                        INNER JOIN tag_query tq ON i.id = tq.id
                    WHERE tq.tag_query MATCH :query
                "})?;
                stmt.query_map(&[(":query", query)], to_item_closure)
            }
            None => {
                stmt = self.conn.prepare(indoc! {"
                    SELECT i.id, i.path, i.tags, i.meta_tags FROM items i
                "})?;
                stmt.query_map([], to_item_closure)
            }
        }?;
        let items: Vec<Item> = mapped_rows.flatten().collect();
        Ok(items)
    }

    fn get_item_by_path(&self, path: impl AsRef<str>) -> Result<Item, DatabaseError> {
        let path = path.as_ref();
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, tags, meta_tags FROM items WHERE path = :path LIMIT 1")?;
        let item = stmt.query_row([&path], |row| {
            Ok(Item {
                id: row.get::<_, i64>(0)?,
                path: row.get::<_, String>(1)?,
                tags: row.get::<_, String>(2)?,
                meta_tags: row.get::<_, String>(3)?,
            })
        });
        if let Err(QueryReturnedNoRows) = item {
            return Err(DatabaseError::ItemNotFound);
        }

        Ok(item?)
    }

    fn get_item_by_id(&self, id: i64) -> Result<Item, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, tags, meta_tags FROM items WHERE id = :id LIMIT 1")?;
        let item = stmt.query_row([id], |row| {
            Ok(Item {
                id: row.get::<_, i64>(0)?,
                path: row.get::<_, String>(1)?,
                tags: row.get::<_, String>(2)?,
                meta_tags: row.get::<_, String>(3)?,
            })
        });
        if let Err(QueryReturnedNoRows) = item {
            return Err(DatabaseError::ItemNotFound);
        }

        Ok(item?)
    }

    fn remove_item_by_path(&self, path: impl AsRef<str>) -> Result<(), DatabaseError> {
        let path = path.as_ref();
        self.conn
            .execute("DELETE FROM items WHERE path = :path", [path])?;
        Ok(())
    }

    fn remove_item_by_id(&self, id: i64) -> Result<(), DatabaseError> {
        self.conn
            .execute("DELETE FROM items WHERE id = :id", [id])?;
        Ok(())
    }

    fn update_tags(&self, item_id: i64, tags: impl AsRef<str>) -> Result<(), DatabaseError> {
        let rv = self.conn.execute(
            "UPDATE items SET tags = :tags WHERE id = :id",
            params![tags.as_ref(), item_id],
        );
        match rv {
            Ok(_) => Ok(()),
            Err(e) => Err(DatabaseError::BackendError(e)),
        }
    }

    fn update_path(&self, item_id: i64, path: impl AsRef<str>) -> Result<(), DatabaseError> {
        let path = path.as_ref();
        let rv = self.conn.execute(
            "UPDATE items SET path = :path WHERE id = :id",
            params![path, item_id],
        );
        match rv {
            Ok(_) => Ok(()),
            Err(e) => Err(DatabaseError::BackendError(e)),
        }
    }
}

lazy_static! {
    #[rustfmt::skip]
    static ref MIGRATIONS: Migrations<'static> =
        Migrations::new(vec![
            M::up(include_str!("migrations/01u_inital.sql"))
            .down(include_str!("migrations/01d_inital.sql")),
        ]);
}

pub fn open_database(db_path: impl AsRef<Path>) -> Result<Connection, OpenError> {
    let db_path = db_path.as_ref();
    let mut conn = Connection::open(db_path).map_err(OpenError::FailedToCreateDatabase)?;

    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();

    MIGRATIONS
        .to_latest(&mut conn)
        .map_err(OpenError::FailedToMigrateDatabase)?;

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use tempfile::{tempdir, TempDir};

    use crate::testutils::assert_unordered_eq;

    use super::*;

    /// The only purpose of this struct is to bundle `Repo` and `TempDir` together. This ensures that
    /// `TempDir` is dropped AFTER `Repo`.
    ///
    /// Otherwise, if `TempDir` drops first, it cannot delete the temp folder as `Repo` is still using
    /// the database.
    struct TestRepo {
        repo: Repo,
        #[allow(dead_code)]
        tempdir: TempDir,
    }

    impl TestRepo {
        fn new() -> Self {
            let dir = tempdir().unwrap();
            let repo = Repo::open(&dir).unwrap();
            Self { repo, tempdir: dir }
        }
    }

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

        assert!(matches!(rv, Err(DatabaseError::DuplicatePathError(_))));
    }

    #[test]
    fn can_query_items() {
        fn expect_query(repo: &Repo, query: &str, expected: Vec<&str>) {
            let items = repo.get_items(Some(query)).unwrap();

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
        let items = repo.get_items(None).unwrap();
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
        assert!(matches!(rv, Err(DatabaseError::ItemNotFound)))
    }

    #[test]
    fn can_remove_item_by_id() {
        let mut tr = testrepo_1();
        let repo = &mut tr.repo;
        repo.remove_item_by_id(1).unwrap();
        let rv = repo.get_item_by_id(1);
        assert!(matches!(rv, Err(DatabaseError::ItemNotFound)))
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
        //     a b -e inpath:1 | d e inpath:0
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
            let paths = scan_dir(r#"D:\Audio Samples\"#).unwrap();
            println!("  Took: {:?}", start.elapsed());

            println!("Adding paths");
            println!("  Inserting {} paths...", paths.len());
            let start = Instant::now();
            repo.insert_items(paths.iter().map(|p| (p.to_str().unwrap(), "asd")))
                .unwrap();
            println!("  Took: {:?}", start.elapsed());
            println!("Done!");
        }
    }
}
