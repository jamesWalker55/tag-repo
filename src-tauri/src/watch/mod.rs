use std::path::Path;

use async_trait::async_trait;
use notify::{Event, RecursiveMode};

pub use crate::watch::windows::ReadDirectoryChangesNormWatcher;

mod windows;

/// A normalised watcher. All watchers that implement this should behave the same across different
/// operating systems. E.g. Same events for file renames, file moves.
#[async_trait]
pub trait NormWatcher {
    /// Add a path to be watched. This should just be a wrapper for the watcher's #watch method
    fn watch(&mut self, path: &Path, recursive_mode: RecursiveMode) -> notify::Result<()>;

    /// Receive the next event. This should just be a wrapper for a receiver's #recv method
    async fn recv(&mut self) -> Option<notify::Result<Event>>;

    #[cfg(test)]
    /// Stop the watcher immediately. After calling this, #recv must not suspend forever and
    /// eventually return None. This is used by the test suite to gather a list of all events
    /// after suspending the watcher.
    fn stop_watching(&mut self);
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::fs;
    use std::fs::File;
    use std::path::PathBuf;
    use std::time::Duration;

    use notify::event::ModifyKind::Name;
    use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
    use notify::EventKind;
    use notify::EventKind::{Create, Modify, Remove};
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
        fn rename(&self, a: &str, b: &str) {
            let a = self.base_path.join(a);
            let b = self.base_path.join(b);
            if b.exists() {
                Err::<(), _>(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    b.to_string_lossy(),
                ))
                .unwrap();
            } else {
                fs::rename(a, b).unwrap();
            }
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

    #[derive(Debug)]
    enum EventsVerifierError {
        NoMoreEvents,
        StillHasEvents,
        PathNotEqual(PathBuf),
        UnexpectedEventType(EventKind),
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
            watcher.stop_watching();
            if ignore_modify_any {
                while let Some(evt) = watcher.recv().await {
                    let evt = evt.unwrap();
                    if let Modify(ModifyKind::Any) = evt.kind {
                        // do nothing
                    } else {
                        events.push_back(evt);
                    }
                }
            } else {
                while let Some(evt) = watcher.recv().await {
                    let evt = evt.unwrap();
                    events.push_back(evt);
                }
            }
            Self {
                base_path: base_path.as_ref().to_path_buf(),
                events,
                ignore_modify_any,
            }
        }
        fn create(&mut self, expected_path: &str) -> Result<(), EventsVerifierError> {
            let expected_path = self.base_path.join(expected_path);

            if self.events.len() == 0 {
                Err(EventsVerifierError::NoMoreEvents)
            } else if let Event { kind: Create(_), paths, .. } = self.events.get(0).unwrap() {
                let path = paths.get(0).unwrap();
                if path.as_path() == expected_path {
                    self.events.pop_front();
                    Ok(())
                } else {
                    Err(EventsVerifierError::PathNotEqual(path.clone()))
                }
            } else {
                let evt = self.events.get(0).unwrap();
                Err(EventsVerifierError::UnexpectedEventType(evt.kind.clone()))
            }
        }
        fn remove(&mut self, expected_path: &str) -> Result<(), EventsVerifierError> {
            let expected_path = self.base_path.join(expected_path);

            if self.events.len() == 0 {
                Err(EventsVerifierError::NoMoreEvents)
            } else if let Event { kind: Remove(_), paths, .. } = self.events.get(0).unwrap() {
                let path = paths.get(0).unwrap();
                if path.as_path() == expected_path {
                    self.events.pop_front();
                    Ok(())
                } else {
                    Err(EventsVerifierError::PathNotEqual(path.clone()))
                }
            } else {
                let evt = self.events.get(0).unwrap();
                Err(EventsVerifierError::UnexpectedEventType(evt.kind.clone()))
            }
        }
        fn rename(
            &mut self,
            expected_a: &str,
            expected_b: &str,
        ) -> Result<(), EventsVerifierError> {
            let expected_a = self.base_path.join(expected_a);
            let expected_b = self.base_path.join(expected_b);

            if self.events.len() == 0 {
                Err(EventsVerifierError::NoMoreEvents)
            } else if let Event { kind: Modify(Name(RenameMode::Both)), paths, .. } =
                self.events.get(0).unwrap()
            {
                let a = paths.get(0).unwrap();
                let b = paths.get(1).unwrap();
                if a.as_path() == expected_a && b.as_path() == expected_b {
                    self.events.pop_front();
                    Ok(())
                } else if a.as_path() != expected_a {
                    Err(EventsVerifierError::PathNotEqual(a.clone()))
                } else if b.as_path() != expected_b {
                    Err(EventsVerifierError::PathNotEqual(b.clone()))
                } else {
                    unreachable!()
                }
            } else {
                let evt = self.events.get(0).unwrap();
                Err(EventsVerifierError::UnexpectedEventType(evt.kind.clone()))
            }
        }
        fn end(&mut self) -> Result<(), EventsVerifierError> {
            if self.events.len() == 0 {
                Ok(())
            } else {
                Err(EventsVerifierError::StillHasEvents)
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
        verify.end().unwrap();
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
        verify.end().unwrap();
    }

    #[tokio::test]
    async fn file_creations_02() {
        let (dir, mut watcher, op) = setup().await;

        op.create("a");
        op.create("b");
        op.create_dir("sub");
        op.create("sub/c");
        op.create("sub/d");

        let mut verify = EventsVerifier::new(dir.path(), watcher, true).await;
        verify.create("a").unwrap();
        verify.create("b").unwrap();
        verify.create("sub").unwrap();
        verify.create("sub/c").unwrap();
        verify.create("sub/d").unwrap();
        verify.end().unwrap();
    }

    #[tokio::test]
    async fn file_removes_01() {
        let (dir, mut watcher, op) = setup().await;

        op.create("a");
        op.create("b");
        op.remove("b");
        op.create_dir("sub");
        op.create("sub/c");
        op.create("sub/d");
        op.remove("sub/c");

        let mut verify = EventsVerifier::new(dir.path(), watcher, true).await;
        verify.create("a").unwrap();
        verify.create("b").unwrap();
        verify.create("sub").unwrap();
        verify.create("sub/c").unwrap();
        verify.create("sub/d").unwrap();
        // because of our rename handling code, remove events are delayed by a few ms
        // so they end up in the end
        verify.remove("b").unwrap();
        verify.remove("sub/c").unwrap();
        verify.end().unwrap();
    }

    #[tokio::test]
    async fn file_removes_02() {
        let (dir, mut watcher, op) = setup().await;

        op.create("a");
        op.create("b");
        op.remove("b");
        op.create_dir("sub");
        op.create("sub/c");
        op.create("sub/d");
        op.remove_dir("sub");

        let mut verify = EventsVerifier::new(dir.path(), watcher, true).await;
        verify.create("a").unwrap();
        verify.create("b").unwrap();
        verify.create("sub").unwrap();
        verify.create("sub/c").unwrap();
        verify.create("sub/d").unwrap();
        verify.remove("b").unwrap();
        verify.remove("sub/c").unwrap();
        verify.remove("sub/d").unwrap();
        verify.remove("sub").unwrap();
        verify.end().unwrap();
    }

    #[tokio::test]
    async fn file_renames_01() {
        let (dir, mut watcher, op) = setup().await;

        op.create("a");
        op.create("b");
        op.create("c");
        op.rename("c", "apple");

        let mut verify = EventsVerifier::new(dir.path(), watcher, true).await;
        verify.create("a").unwrap();
        verify.create("b").unwrap();
        verify.create("c").unwrap();
        verify.rename("c", "apple").unwrap();
        verify.end().unwrap();
    }

    #[tokio::test]
    async fn file_renames_02() {
        let (dir, mut watcher, op) = setup().await;

        op.create("a");
        op.create("b");
        op.create("c");
        op.create_dir("sub");
        op.create("sub/a");
        op.create("sub/b");
        op.rename("sub/a", "sub/hello");

        let mut verify = EventsVerifier::new(dir.path(), watcher, true).await;
        verify.create("a").unwrap();
        verify.create("b").unwrap();
        verify.create("c").unwrap();
        verify.create("sub").unwrap();
        verify.create("sub/a").unwrap();
        verify.create("sub/b").unwrap();
        verify.rename("sub/a", "sub/hello").unwrap();
        verify.end().unwrap();
    }

    #[tokio::test]
    async fn file_renames_03() {
        let (dir, mut watcher, op) = setup().await;

        op.create("a");
        op.create("b");
        op.create("c");
        op.create_dir("sub");
        op.create("sub/a");
        op.create("sub/b");
        op.rename("sub", "hello");

        let mut verify = EventsVerifier::new(dir.path(), watcher, true).await;
        verify.create("a").unwrap();
        verify.create("b").unwrap();
        verify.create("c").unwrap();
        verify.create("sub").unwrap();
        verify.create("sub/a").unwrap();
        verify.create("sub/b").unwrap();
        verify.rename("sub", "hello").unwrap();
        verify.end().unwrap();
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
