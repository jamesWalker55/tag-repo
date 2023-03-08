use notify::event::ModifyKind::Name;
use notify::event::{ModifyKind, RenameMode};
use notify::EventKind::Modify;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::runtime::Handle;
use tokio::sync::mpsc::unbounded_channel;

async fn async_watch(path: impl AsRef<Path>) -> notify::Result<()> {
    let (tx, mut rx) = unbounded_channel();
    let tokio_handle = Handle::current();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            tokio_handle.block_on(async {
                tx.send(res).unwrap();
            })
        },
        Config::default(),
    )?;

    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    let mut last_rename_from: Option<PathBuf> = None;

    while let Some(evt) = rx.recv().await {
        let evt = evt.unwrap();
        if let Event {
            kind: Modify(Name(RenameMode::From)),
            mut paths,
            ..
        } = evt
        {
            let path = paths.pop().unwrap();
            last_rename_from = Some(path);
            continue;
        } else if let Event {
            kind: Modify(Name(RenameMode::To)),
            mut paths,
            ..
        } = evt
        {
            let from_path = last_rename_from
                .take()
                .expect("Got 'Rename To' event, but no 'Rename From' event happened before this!");
            let to_path = paths.pop().unwrap();
            let evt = Event {
                kind: Modify(Name(RenameMode::Both)),
                paths: vec![from_path, to_path],
                attrs: evt.attrs.clone(),
            };
            println!("{:?}", evt);
        } else if let Event {
            kind: Modify(ModifyKind::Any),
            ..
        } = evt
        {
            // ignore
            // println!("{:?}", evt);
        } else {
            println!("{:?}", evt);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let path = r"D:\Programming\rust-learning\temp";
    println!("watching {}", path);

    if let Err(e) = async_watch(path).await {
        println!("error: {:?}", e)
    }
}
