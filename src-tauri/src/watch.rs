use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
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

    while let Some(res) = rx.recv().await {
        let res = res.unwrap();
        println!("{:?}", res);
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
