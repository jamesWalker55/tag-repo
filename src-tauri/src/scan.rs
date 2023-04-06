use path_slash::{PathBufExt, PathExt};
use relative_path::{RelativePath, RelativePathBuf};
use std::fs;
use std::fs::DirEntry;
use std::io::Error;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum ScanError {
    #[error("cannot scan path, it is not a directory")]
    NotADirectory,
    #[error("IOError occured when trying to scan the given path, {0}")]
    IOError(Error),
}

#[derive(Debug)]
pub struct Options {
    /// Ignored paths, relative to the root folder.
    excluded_paths: Vec<RelativePathBuf>,
    /// Ignored filenames, these are checked in all subfolders.
    excluded_names: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            excluded_paths: vec![RelativePathBuf::from(".tagrepo")],
            excluded_names: vec![],
        }
    }
}

/// Scan a given folder, return a vector of paths `Vec<PathBuf>`
#[tracing::instrument(skip(path), fields(path = path.as_ref().to_string_lossy().to_string()))]
pub fn scan_dir(
    path: impl AsRef<Path>,
    options: Options,
) -> Result<Vec<RelativePathBuf>, ScanError> {
    let path = path.as_ref();

    // make sure path is a directory
    let metadata = path.metadata().map_err(ScanError::IOError)?;
    if !metadata.is_dir() {
        return Err(ScanError::NotADirectory);
    }

    let mut items = vec![];
    let mut unscanned_dirs = vec![];

    // scan the path for initial list of folders
    let dir_iter = fs::read_dir(path).map_err(ScanError::IOError)?;
    classify_dir_items(dir_iter, &mut items, &mut unscanned_dirs, &path, &options);

    // scan remaining folders
    while !unscanned_dirs.is_empty() {
        match fs::read_dir(unscanned_dirs.pop().unwrap()) {
            Ok(dir_iter) => {
                classify_dir_items(dir_iter, &mut items, &mut unscanned_dirs, &path, &options)
            }
            Err(err) => warn!("Failed to scan folder: {}", err),
        }
    }

    Ok(items)
}

enum PathType {
    Item(RelativePathBuf),
    Directory(PathBuf),
    Ignored,
}

fn classify_path(path: PathBuf, root_path: &Path, options: &Options) -> PathType {
    let is_dir = match fs::metadata(&path) {
        Ok(metadata) => metadata.is_dir(),
        Err(err) => {
            warn!("Failed to get path metadata, treating as file: {:?}", err);
            false
        }
    };

    // convert to relative path
    let relpath = &path
        .strip_prefix(root_path)
        .expect("failed to strip prefix from path");
    let relpath =
        RelativePathBuf::from_path(relpath).expect("failed to convert to RelativePathBuf");

    if options.excluded_paths.contains(&relpath) {
        debug!("Skipping excluded path: {}", relpath);
        return PathType::Ignored;
    }

    let file_name = relpath.file_name().expect("path doesn't have file name");
    if options
        .excluded_names
        .iter()
        .any(|name| name.as_str() == file_name)
    {
        debug!("Skipping excluded file name: {}", relpath);
        return PathType::Ignored;
    }

    if is_dir {
        PathType::Directory(path)
    } else {
        PathType::Item(relpath)
    }
}

/// Classify incoming DirEntries as either items or folders to be further scanned.
fn classify_dir_items<T>(
    dir_iter: T,
    items: &mut Vec<RelativePathBuf>,
    unscanned_dirs: &mut Vec<PathBuf>,
    root_path: &Path,
    options: &Options,
) where
    T: Iterator<Item = Result<DirEntry, Error>>,
{
    for entry in dir_iter {
        let Ok(entry) = entry else {
            warn!("Failed to scan entry: {:?}", entry);
            continue;
        };

        match classify_path(entry.path(), root_path, &options) {
            PathType::Item(path) => {
                items.push(path);
            }
            PathType::Directory(path) => {
                unscanned_dirs.push(path);
            }
            PathType::Ignored => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs::File;
    use std::time::Instant;

    use tempfile::{tempdir, TempDir};

    use crate::tests::utils::assert_unordered_eq;

    use super::*;

    fn test_folder_1() -> TempDir {
        let dir = tempdir().unwrap();

        let paths_to_create = vec![
            dir.path().join("apple"),
            dir.path().join("bee"),
            dir.path().join("cat"),
        ];
        for p in &paths_to_create {
            File::create(p).unwrap();
        }

        dir
    }

    #[test]
    fn scans_files_in_folder() {
        let dir = test_folder_1();

        let expected = vec!["apple", "bee", "cat"];

        let scanned_paths = scan_dir(dir, Options::default()).unwrap();

        assert_unordered_eq(scanned_paths.iter().map(|x| x.as_str()), expected)
    }

    #[test]
    fn ignores_files_in_folder() {
        let dir = test_folder_1();

        let mut options = Options::default();
        options.excluded_paths.push(RelativePathBuf::from("apple"));

        let expected = vec!["bee", "cat"];

        let scanned_paths = scan_dir(dir, options).unwrap();

        assert_unordered_eq(scanned_paths.iter().map(|x| x.as_str()), expected)
    }

    #[test]
    fn set_benchmark() -> () {
        let path = PathBuf::from(r#"D:\Audio Samples\"#);
        let start = Instant::now();
        let paths = scan_dir(path, Options::default()).unwrap();
        let duration = start.elapsed();
        println!("Time elapsed for scan: {:?}", duration);
        println!("Number of paths: {}", paths.len());

        let start = Instant::now();
        let paths: HashSet<String> = HashSet::from_iter(paths.iter().map(|x| x.to_string()));
        let duration = start.elapsed();
        println!("Time elapsed for set: {:?}", duration);
        println!("Number of paths: {}", paths.len());
    }

    #[test]
    fn benchmark() -> () {
        let path = PathBuf::from(r#"D:\Audio Samples\"#);
        let start = Instant::now();
        let r = scan_dir(path, Options::default());
        let duration = start.elapsed();

        println!("Time elapsed: {:?}", duration);

        match r {
            Ok(items) => {
                println!("Items: {}", items.len());
                // 151293
            }
            Err(e) => {
                dbg!(e);
            }
        }
    }
}
