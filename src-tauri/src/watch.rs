use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver},
    SinkExt, StreamExt,
};
use notify::event::ModifyKind::Name;
use notify::EventKind::Modify;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};

// #[derive(Debug)]
// pub enum FSEvent {
//   /// File creation event. This should never be used for file renames.
//   Create(PathBuf),
//   /// File removal event. This should never be used for file renames.
//   Remove(PathBuf),
//   /// File rename event.
//   Rename(PathBuf, PathBuf),
//   // Modify(PathBuf),
// }

fn async_watcher() -> notify::Result<(RecommendedWatcher, UnboundedReceiver<notify::Result<Event>>)>
{
    let (mut tx, rx) = unbounded();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                tx.send(res).await.unwrap();
            })
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}

async fn async_watch(path: impl AsRef<Path>) -> notify::Result<()> {
    use notify::EventKind::*;

    let (mut watcher, mut rx) = async_watcher()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    let last_rename_from: Option<PathBuf> = None;

    while let Some(res) = rx.next().await {
        let fsevent: Option<FSEvent> = match res {
            Ok(Event {
                kind: Create(kind),
                paths,
                attrs,
            }) => {
                let path = paths.get(0)?;
            }
            // Ok(event) => {
            //   let action = match event.kind {
            //     EventKind::Any => "Any",
            //     EventKind::Access(access_kind) => {
            //       access_kind
            //       "Access"
            //     },
            //     EventKind::Create(create_kind) => {
            //       "Create"
            //     },
            //     EventKind::Modify(modify_kind) => {
            //       "Modify"
            //     },
            //     EventKind::Remove(remove_kind) => {
            //       "Remove"
            //     },
            //     EventKind::Other => "Other",
            //   };
            //   println!("changed: {:?}", event)
            // },
        };
        // print the fsevent if it exists
        match fsevent {
            Some(fsevent) => println!("{:?}", fsevent),
            None => (),
        }
    }

    Ok(())
}

/// Async, futures channel based event watching
fn main() {
    let path = r"D:\Programming\rust-learning\temp";
    println!("watching {}", path);

    futures::executor::block_on(async {
        if let Err(e) = async_watch(path).await {
            println!("error: {:?}", e)
        }
    });
}
