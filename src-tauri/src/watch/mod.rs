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
    use std::collections::VecDeque;
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

    struct EventsVerifier {
        base_path: PathBuf,
        events: VecDeque<Event>,
        ignore_modify_any: bool,
    }

    impl EventsVerifier {
        async fn new(
            base_path: impl AsRef<Path>,
            mut watcher: impl NormWatcher,
            ignore_modify_any: bool,
        ) -> Self {
            let mut events = VecDeque::new();
            while let Ok(Some(Ok(evt))) = timeout(Duration::from_millis(10), watcher.recv()).await {
                events.push_back(evt);
            }
            println!("Created verifier with events: {:?}", events);
            Self {
                base_path: base_path.as_ref().to_path_buf(),
                events,
                ignore_modify_any,
            }
        }
        fn discard_modify_anys(&mut self) {
            while self.events.len() > 0 {
                if let Event { kind: Modify(ModifyKind::Any), .. } = self.events.get(0).unwrap() {
                    println!("Discarded modify::any");
                    self.events.pop_front();
                } else {
                    break;
                }
            }
        }
        fn create(&mut self, relpath: &str) -> Result<(), ()> {
            let relpath = PathBuf::from(relpath);
            if self.ignore_modify_any {
                self.discard_modify_anys();
            }
            if self.events.len() == 0 {
                Err(())
            } else if let Event { kind: Create(_), paths, .. } = self.events.get(0).unwrap() {
                let path = paths.get(0).unwrap();
                let expected_path = self.base_path.join(relpath);
                if path.as_path() == expected_path {
                    self.events.pop_front();
                    Ok(())
                } else {
                    Err(())
                }
            } else {
                Err(())
            }
        }
        fn remove(&mut self, relpath: &str) -> Result<(), ()> {
            let relpath = PathBuf::from(relpath);
            if self.ignore_modify_any {
                self.discard_modify_anys();
            }
            if self.events.len() == 0 {
                Err(())
            } else if let Event { kind: Remove(_), paths, .. } = self.events.get(0).unwrap() {
                let path = paths.get(0).unwrap();
                let expected_path = self.base_path.join(relpath);
                if path.as_path() == expected_path {
                    self.events.pop_front();
                    Ok(())
                } else {
                    Err(())
                }
            } else {
                Err(())
            }
        }
    }

    async fn setup() -> (TempDir, ReadDirectoryChangesNormWatcher, FSOperator) {
        // the temporary directory to test in
        let dir = tempdir().unwrap();

        // watcher for directory
        let mut watcher = ReadDirectoryChangesNormWatcher::new().unwrap();
        watcher.watch(dir.path(), RecursiveMode::Recursive).unwrap();

        // operator on temporary directory
        let op = FSOperator::new(dir.path());

        // also return the temp dir so it doesn't get dropped when this function ends
        (dir, watcher, op)
    }

    #[tokio::test]
    async fn basic_test() {
        let (dir, mut watcher, op) = setup().await;

        op.create("hello world");

        let mut verify = EventsVerifier::new(dir.path(), watcher, true).await;
        verify.create("hello world").unwrap();
    }

    #[tokio::test]
    async fn file_creations_01() {
        let (dir, mut watcher, op) = setup().await;

        op.create("a");
        op.create("b");
        op.create("c");

        let mut verify = EventsVerifier::new(dir.path(), watcher, true).await;
        verify.create("a").unwrap();
        verify.create("b").unwrap();
        verify.create("c").unwrap();
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
