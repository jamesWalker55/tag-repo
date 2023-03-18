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
    use tokio::runtime::Handle;

    use super::*;

    /// Run this test with `--nocapture` to see the output
    #[tokio::test]
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
                    println!("Moved  : {:?}", paths.pop().unwrap());
                    println!("      -> {:?}", paths.pop().unwrap());
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
