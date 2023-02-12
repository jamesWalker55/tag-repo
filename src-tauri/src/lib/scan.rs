use std::fs;
use std::fs::ReadDir;
use std::io::Error;
use std::path::{Path, PathBuf};

// pub enum Filter {
//   ExcludeName(PathBuf),
//   ExcludeContainsItem(PathBuf),
// }

#[derive(Debug)]
pub enum ScanError {
  NotADirectory,
  IOError(Error),
}

pub fn scan_dir(path: PathBuf) -> Result<Vec<PathBuf>, ScanError> {
  // make sure path is a directory
  let metadata = path.metadata().map_err(|e| ScanError::IOError(e))?;
  if !metadata.is_dir() { return Err(ScanError::NotADirectory); }

  let mut items = vec![];
  let mut unscanned_dirs = vec![];

  // scan the path for initial list of folders
  let dir_iter = fs::read_dir(&path).map_err(|e| ScanError::IOError(e))?;
  classify_dir_items(dir_iter, &mut items, &mut unscanned_dirs);

  // scan remaining folders
  while unscanned_dirs.len() > 0 {
    if let Ok(dir_iter) = fs::read_dir(&unscanned_dirs.pop().unwrap()) {
      classify_dir_items(dir_iter, &mut items, &mut unscanned_dirs);
    }
  }

  Ok(items)
}

fn classify_dir_items(dir_iter: ReadDir, items: &mut Vec<PathBuf>, unscanned_dirs: &mut Vec<PathBuf>) {
  for result in dir_iter {
    if let Ok(entry) = result {
      if let Ok(metadata) = entry.metadata() {
        if metadata.is_dir() {
          unscanned_dirs.push(entry.path());
        } else {
          items.push(entry.path());
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Instant;

  #[test]
  fn benchmark() -> () {
    let path = PathBuf::from("D:/");
    let start = Instant::now();
    let r = scan_dir(path);
    let duration = start.elapsed();

    println!("Time elapsed: {:?}", duration);

    match r {
      Ok(items) => {
        println!("Items: {}", items.len());
      }
      Err(e) => { dbg!(e); }
    }
  }
}
