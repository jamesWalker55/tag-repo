use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver},
    SinkExt, StreamExt,
};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;

async fn async_watch(path: impl AsRef<Path>) -> notify::Result<()> {
    let (mut tx, mut rx) = unbounded();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                tx.send(res).await.unwrap();
            })
        },
        Config::default(),
    )?;

    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    while let Some(res) = rx.next().await {
        let res = res.unwrap();
        println!("{:?}", res);
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let path = r"C:\Files\temp\fs";
    println!("watching {}", path);

    if let Err(e) = async_watch(path).await {
        println!("error: {:?}", e)
    }
}
