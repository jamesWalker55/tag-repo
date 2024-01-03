// The following code is derived from:
// https://github.com/ivanceras/r2d2-sqlite

use rusqlite::OpenFlags;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio_rusqlite::{Connection, Error};
use uuid::Uuid;

#[derive(Debug)]
enum Source {
    File(PathBuf),
    Memory(String),
}

type InitFn = dyn Fn(&mut rusqlite::Connection) -> Result<(), Error> + Send + Sync + 'static;

/// An `r2d2::ManageConnection` for `rusqlite::Connection`s.
pub struct SqliteConnectionManager {
    source: Source,
    flags: OpenFlags,
    init: Option<Arc<InitFn>>,
    _persist: Mutex<Option<Connection>>,
}

impl fmt::Debug for SqliteConnectionManager {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut builder = f.debug_struct("SqliteConnectionManager");
        let _ = builder.field("source", &self.source);
        let _ = builder.field("flags", &self.source);
        let _ = builder.field("init", &self.init.as_ref().map(|_| "InitFn"));
        builder.finish()
    }
}

impl SqliteConnectionManager {
    /// Creates a new `SqliteConnectionManager` from file.
    ///
    /// See `rusqlite::Connection::open`
    pub fn file<P: AsRef<Path>>(path: P) -> Self {
        Self {
            source: Source::File(path.as_ref().to_path_buf()),
            flags: OpenFlags::default(),
            init: None,
            _persist: Mutex::new(None),
        }
    }

    /// Creates a new `SqliteConnectionManager` from memory.
    pub fn memory() -> Self {
        Self {
            source: Source::Memory(Uuid::new_v4().to_string()),
            flags: OpenFlags::default(),
            init: None,
            _persist: Mutex::new(None),
        }
    }

    /// Converts `SqliteConnectionManager` into one that sets OpenFlags upon
    /// connection creation.
    ///
    /// See `rustqlite::OpenFlags` for a list of available flags.
    pub fn with_flags(self, flags: OpenFlags) -> Self {
        Self { flags, ..self }
    }

    pub fn with_init<F>(self, init: F) -> Self
    where
        F: Fn(&mut rusqlite::Connection) -> Result<(), Error> + Send + Sync + 'static,
    {
        let init: Option<Arc<InitFn>> = Some(Arc::new(init));
        Self { init, ..self }
    }
}

impl r2d2::ManageConnection for SqliteConnectionManager {
    type Connection = Connection;
    type Error = Error;

    fn connect(&self) -> Result<Connection, Self::Error> {
        match self.source {
            Source::File(ref path) => {
                tauri::async_runtime::block_on(Connection::open_with_flags(path, self.flags))
            }
            Source::Memory(ref id) => {
                let connection = || {
                    tauri::async_runtime::block_on(Connection::open_with_flags(
                        format!("file:{}?mode=memory&cache=shared", id),
                        self.flags,
                    ))
                };

                {
                    let mut persist = self._persist.lock().unwrap();
                    if persist.is_none() {
                        *persist = Some(connection()?);
                    }
                }

                connection()
            }
        }
        .map_err(Into::into)
        .and_then(|mut conn| match self.init {
            None => Ok(conn),
            Some(ref init) => tauri::async_runtime::block_on(async {
                let init = init.clone();
                conn.call(move |conn| init(conn)).await
            })
            .map(|_| conn),
        })
    }

    fn is_valid(&self, conn: &mut Connection) -> Result<(), Self::Error> {
        tauri::async_runtime::block_on(conn.call(|conn| {
            conn.execute_batch("")
                .map_err(|x| tokio_rusqlite::Error::Rusqlite(x))
        }))
    }

    fn has_broken(&self, _: &mut Connection) -> bool {
        false
    }
}
