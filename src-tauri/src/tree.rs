use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};

use serde::{Serialize, Serializer};
use thiserror::Error;

struct Folder<'a> {
    children: HashMap<&'a str, Folder<'a>>,
}

impl<'a> Serialize for Folder<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.children.serialize(serializer)
    }
}

impl<'a> Debug for Folder<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.children.fmt(f)
    }
}

impl<'a> Folder<'a> {
    fn new() -> Self {
        Self { children: HashMap::new() }
    }

    fn to_folder_buf(&self) -> FolderBuf {
        let mut map = HashMap::with_capacity(self.children.len());
        for (dirname, folder) in &self.children {
            map.insert(dirname.to_string(), folder.to_folder_buf());
        }
        FolderBuf { children: map }
    }
}

pub struct FolderBuf {
    children: HashMap<String, FolderBuf>,
}

impl Serialize for FolderBuf {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.children.serialize(serializer)
    }
}

impl Debug for FolderBuf {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.children.fmt(f)
    }
}

#[derive(Error, Debug)]
pub enum PathTreeError {
    #[error("malformed path, {0}")]
    MalformedPath(PathBuf),
}

/// Construct a tree from a given list of paths.
/// The paths must already be sorted alphabetically.
pub fn from_ordered_paths(paths: &Vec<impl AsRef<Path>>) -> Result<FolderBuf, PathTreeError> {
    let mut root = Folder::new();
    for path in paths.iter() {
        let path = path.as_ref();
        let mut current_folder = &mut root;

        for component in path.components().map(|comp| {
            Ok(comp
                .as_os_str()
                .to_str()
                .ok_or(PathTreeError::MalformedPath(path.to_path_buf()))?)
        }) {
            let component = component?;
            current_folder = current_folder
                .children
                .entry(component)
                .or_insert_with(|| Folder::new());
        }
    }

    Ok(root.to_folder_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_test() {
        let paths = vec![
            "",
            "Band",
            "Band/ready",
            "Guitar IRs",
            "Guitar IRs/Aurora DSP/FREE PACK",
            "Guitar IRs/Aurora DSP/FREE PACK/WAVE/GOVERNOR/LEWITT 0cm",
            "Guitar IRs/Aurora DSP/FREE PACK/WAVE/GOVERNOR/LEWITT 2cm",
            "Guitar IRs/Aurora DSP/FREE PACK/WAVE/GOVERNOR/LEWITT 4cm",
        ];
        let dirs = from_ordered_paths(&paths).unwrap();
        dbg!(&dirs);
    }
}
