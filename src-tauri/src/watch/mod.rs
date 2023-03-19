use std::path::Path;

use async_trait::async_trait;
use notify::{Event, RecursiveMode};

use crate::watch::windows::ReadDirectoryChangesNormWatcher;

mod windows;

/// A normalised watcher. All watchers that implement this should behave the same across different
/// operating systems. E.g. Same events for file renames, file moves.
#[async_trait]
pub trait NormWatcher {
    /// Add a path to be watched. This should just be a wrapper for the watcher's #watch method
    fn watch(&mut self, path: &Path, recursive_mode: RecursiveMode) -> notify::Result<()>;

    /// Receive the next event. This should just be a wrapper for a receiver's #recv method
    async fn recv(&mut self) -> Option<notify::Result<Event>>;
}

#[cfg(test)]
mod tests {
    use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
    use notify::EventKind::{Create, Modify, Remove};
    use std::fs;
    use std::fs::File;
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::{tempdir, TempDir};
    use tokio::time::timeout;

    use super::*;

    struct FSOperator {
        base_path: PathBuf,
    }

    impl FSOperator {
        fn new(base_path: impl AsRef<Path>) -> Self {
            Self { base_path: base_path.as_ref().to_path_buf() }
        }
        fn create(&self, path: &str) {
            let dest = self.base_path.join(path);
            File::create(dest).unwrap();
        }
        fn remove(&self, path: &str) {
            let dest = self.base_path.join(path);
            fs::remove_file(dest).unwrap();
        }
        fn create_dir(&self, path: &str) {
            let dest = self.base_path.join(path);
            fs::create_dir(dest).unwrap();
        }
        fn remove_dir(&self, path: &str) {
            let dest = self.base_path.join(path);
            fs::remove_dir_all(dest).unwrap();
        }
    }

    struct Consumer<T>
    where
        T: NormWatcher,
    {
        base_path: PathBuf,
        watcher: T,
        ignore_modify_any: bool,
        current_event: Option<notify::Result<Event>>,
        event_timeout: Option<Duration>,
    }

    impl<T: NormWatcher> Consumer<T> {
        async fn new(
            base_path: impl AsRef<Path>,
            watcher: T,
            ignore_modify_any: bool,
            event_timeout: Option<Duration>,
        ) -> Self {
            Self {
                base_path: base_path.as_ref().to_path_buf(),
                watcher,
                ignore_modify_any,
                current_event: None,
                event_timeout,
            }
        }
        async fn get_next_event(&mut self) {
            self.current_event = match self.event_timeout {
                Some(duration) => timeout(duration, self.watcher.recv())
                    .await
                    .expect("Failed to get event, timeout"),
                None => self.watcher.recv().await,
            }
        }
        async fn discard_modify_anys(&mut self) {
            while let Some(Ok(Event { kind: Modify(ModifyKind::Any), .. })) = self.current_event {
                println!("Discarded modify::any");
                self.get_next_event().await
            }
        }
        async fn create(&mut self, relpath: &str) -> Result<(), ()> {
            let relpath = PathBuf::from(relpath);
            if self.ignore_modify_any {
                self.discard_modify_anys().await;
            }
            dbg!(&self.current_event);
            if let Some(Ok(Event { kind: Create(_), paths, .. })) = &self.current_event {
                let path = paths.get(0).unwrap();
                let expected_path = self.base_path.join(relpath);
                if path.as_path() == expected_path {
                    Ok(())
                } else {
                    Err(())
                }
            } else {
                Err(())
            }
        }
        async fn remove(&mut self, relpath: &str) -> Result<(), ()> {
            let relpath = PathBuf::from(relpath);
            if self.ignore_modify_any {
                self.discard_modify_anys().await;
            }
            if let Some(Ok(Event { kind: Remove(_), paths, .. })) = &self.current_event {
                let path = paths.get(0).unwrap();
                let expected_path = self.base_path.join(relpath);
                if path.as_path() == expected_path {
                    Ok(())
                } else {
                    Err(())
                }
            } else {
                Err(())
            }
        }
    }

    async fn setup() -> (
        TempDir,
        Consumer<ReadDirectoryChangesNormWatcher>,
        FSOperator,
    ) {
        // the temporary directory to test in
        let dir = tempdir().unwrap();

        // watcher for directory
        let mut watcher = ReadDirectoryChangesNormWatcher::new().unwrap();
        watcher.watch(dir.path(), RecursiveMode::Recursive).unwrap();

        // tester for watcher
        let consume = Consumer::new(dir.path(), watcher, true, Some(Duration::from_secs(1))).await;

        // operator on temporary directory
        let op = FSOperator::new(dir.path());

        // also return the temp dir so it doesn't get dropped when this function ends
        (dir, consume, op)
    }

    #[tokio::test]
    async fn basic_test() {
        let (_dir, mut consume, op) = setup().await;

        op.create("hello world");

        consume.get_next_event().await;
        consume.create("hello world").await.unwrap();
    }

    /// Run this test with `--nocapture` to see the output
    // #[tokio::test]
    async fn watch_a_path() {
        let path = r"D:\Programming\rust-learning\temp";
        println!("watching {}", path);

        let mut watcher = ReadDirectoryChangesNormWatcher::new().unwrap();
        watcher
            .watch(path.as_ref(), RecursiveMode::Recursive)
            .unwrap();

        // Loop through events
        while let Some(evt) = watcher.recv().await {
            let evt = evt.unwrap();
            match evt {
                Event {
                    kind: Modify(ModifyKind::Name(RenameMode::Both)),
                    mut paths,
                    ..
                } => {
                    let dest = paths.pop().unwrap();
                    let src = paths.pop().unwrap();
                    println!("Moved  : {:?}", src);
                    println!("      -> {:?}", dest);
                }
                Event { kind: Modify(ModifyKind::Any), mut paths, .. } => {
                    println!("       m {:?}", paths.pop().unwrap());
                }
                Event { kind: Remove(RemoveKind::Any), mut paths, .. } => {
                    println!("Deleted: {:?}", paths.pop().unwrap());
                }
                Event { kind: Create(CreateKind::Any), mut paths, .. } => {
                    println!("Created: {:?}", paths.pop().unwrap());
                }
                evt => {
                    println!("UNKNOWN EVENT: {:?}", evt)
                }
            }
        }
    }
}
