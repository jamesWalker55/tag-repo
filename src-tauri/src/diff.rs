use futures::StreamExt;
use relative_path::{RelativePath, RelativePathBuf};

use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiffError {
    #[error("failed to diff invalid path, {0}")]
    InvalidPath(RelativePathBuf),
}

#[derive(Debug)]
pub(crate) struct DiffPaths<'a> {
    pub(crate) created: Vec<&'a RelativePath>,
    pub(crate) deleted: Vec<&'a RelativePath>,
    pub(crate) renamed: Vec<(&'a RelativePath, &'a RelativePath)>,
}

impl<'a> DiffPaths<'a> {
    fn new() -> Self {
        Self { created: vec![], deleted: vec![], renamed: vec![] }
    }
}

fn paths_similarity(path1: &RelativePath, path2: &RelativePath) -> i32 {
    // common components from root path
    let mut forward_similarity = 0;
    // whether all path components have been checked
    // if we break early in the loop, then we haven't checked all elements
    let mut path_not_exhausted = false;
    for (a, b) in path1.components().zip(path2.components()) {
        if a == b {
            forward_similarity += 1;
        } else {
            path_not_exhausted = true;
            break;
        }
    }
    if path_not_exhausted {
        // haven't checked all components yet
        // start checking from the back now
        // common components backwards starting from filename
        let mut backwards_similarity = 0;
        for (a, b) in path1.components().rev().zip(path2.components().rev()) {
            if a == b {
                backwards_similarity += 1;
            } else {
                break;
            }
        }
        forward_similarity + backwards_similarity
    } else {
        forward_similarity
    }
}

fn path_diff_to_name_map<'a>(
    paths: impl IntoIterator<Item = &'a RelativePath>,
) -> Result<HashMap<&'a str, Vec<&'a RelativePath>>, DiffError> {
    let mut map: HashMap<&str, Vec<&RelativePath>> = HashMap::new();
    for path in paths {
        let file_name = path
            .file_name()
            .ok_or(DiffError::InvalidPath(path.to_relative_path_buf()))?;
        match map.get_mut(file_name) {
            Some(paths) => paths.push(path),
            None => {
                map.insert(file_name, vec![path]);
            }
        }
    }
    Ok(map)
}

pub(crate) fn diff_path_list<'a>(
    before: &'a HashSet<RelativePathBuf>,
    after: &'a HashSet<RelativePathBuf>,
) -> Result<DiffPaths<'a>, DiffError> {
    let deleted_map =
        path_diff_to_name_map(before.difference(&after).into_iter().map(|x| x.as_ref()))?;
    let mut created_map =
        path_diff_to_name_map(after.difference(&before).into_iter().map(|x| x.as_ref()))?;
    let mut diff = DiffPaths::new();
    for (deleted_file_name, deleted_paths) in &deleted_map {
        let Some(created_paths) = created_map.get_mut(deleted_file_name) else {
            diff.deleted.extend(deleted_paths.into_iter().cloned());
            continue;
        };
        for deleted_path in deleted_paths {
            if created_paths.is_empty() {
                diff.deleted.push(deleted_path);
                break;
            }
            // find closest match in the list of created paths
            let mut best_match = None;
            for (i, created_path) in created_paths.iter().enumerate() {
                let similarity = paths_similarity(deleted_path, created_path);
                match best_match {
                    Some((_, prev_similarity)) => {
                        if similarity > prev_similarity {
                            best_match = Some((i, similarity));
                        }
                    }
                    None => best_match = Some((i, similarity)),
                }
            }
            match best_match {
                Some((i, _)) => {
                    let created_path = created_paths.remove(i);
                    diff.renamed.push((deleted_path, created_path));
                }
                None => {
                    // created_paths is now empty, break this loop since there are no more paths
                    // to match
                    diff.deleted.push(deleted_path);
                    break;
                }
            }
        }
    }
    for paths in created_map.values() {
        for path in paths {
            diff.created.push(path);
        }
    }
    Ok(diff)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_paths_similarity(a: &str, b: &str, similarity: i32) {
        let a = RelativePathBuf::from(a);
        let b = RelativePathBuf::from(b);
        assert_eq!(paths_similarity(&a, &b), similarity);
    }

    #[test]
    fn simpaths_1() {
        assert_paths_similarity("a/b/c", "a/b/c", 3);
    }

    #[test]
    fn simpaths_2() {
        assert_paths_similarity("a/b/cvghsacvsgha", "a/b/c", 2);
    }

    #[test]
    fn simpaths_3() {
        assert_paths_similarity("a/b/cvghsacvsgha/c", "a/b/c", 3);
    }

    #[test]
    fn simpaths_4() {
        assert_paths_similarity("a/b/cvghsacvsgha/d/e", "a/b/q/w/e", 3);
    }

    fn assert_diff_paths(
        input: (Vec<&str>, Vec<&str>),
        output: (Vec<&str>, Vec<&str>, Vec<(&str, &str)>),
    ) {
        let before: HashSet<_> = input
            .0
            .into_iter()
            .map(|x| RelativePathBuf::from(x))
            .collect();
        let after: HashSet<_> = input
            .1
            .into_iter()
            .map(|x| RelativePathBuf::from(x))
            .collect();
        let expected_created: HashSet<_> = output
            .0
            .into_iter()
            .map(|x| RelativePathBuf::from(x))
            .collect();
        let expected_deleted: HashSet<_> = output
            .1
            .into_iter()
            .map(|x| RelativePathBuf::from(x))
            .collect();
        let expected_renamed: HashSet<_> = output
            .2
            .into_iter()
            .map(|(a, b)| (RelativePathBuf::from(a), RelativePathBuf::from(b)))
            .collect();
        let diff = diff_path_list(&before, &after).expect("failed to diff pathlist");
        let created: HashSet<_> = diff
            .created
            .into_iter()
            .map(|x| x.to_relative_path_buf())
            .collect();
        let deleted: HashSet<_> = diff
            .deleted
            .into_iter()
            .map(|x| x.to_relative_path_buf())
            .collect();
        let renamed: HashSet<_> = diff
            .renamed
            .into_iter()
            .map(|(a, b)| (a.to_relative_path_buf(), b.to_relative_path_buf()))
            .collect();
        assert_eq!(expected_created, created, "created paths differ");
        assert_eq!(expected_deleted, deleted, "deleted paths differ");
        assert_eq!(expected_renamed, renamed, "renamed paths differ");
    }

    #[test]
    fn diff_1() {
        assert_diff_paths(
            (vec!["a", "b", "c", "d"], vec!["a", "b", "c", "d"]),
            (vec![], vec![], vec![]),
        )
    }

    #[test]
    fn diff_2() {
        assert_diff_paths(
            (vec!["a", "b", "c"], vec!["a", "b", "c", "d"]),
            (vec!["d"], vec![], vec![]),
        )
    }

    #[test]
    fn diff_3() {
        assert_diff_paths(
            (vec!["a", "b", "c", "d"], vec!["a", "b", "c"]),
            (vec![], vec!["d"], vec![]),
        )
    }

    #[test]
    fn diff_4() {
        assert_diff_paths(
            (vec!["a", "b", "c", "d"], vec!["a", "b", "c", "qwe/d"]),
            (vec![], vec![], vec![("d", "qwe/d")]),
        )
    }

    #[test]
    fn diff_5() {
        assert_diff_paths(
            (
                vec![
                    "zebra.txt",
                    "a/ant.txt",
                    "a/bee.txt",
                    "b/cat.txt",
                    "b/dog.txt",
                    "c/egg.txt",
                    "c/fish.txt",
                    "c/goat.txt",
                ],
                vec![
                    "unicorn.txt",
                    "a/ant.txt",
                    "bee.txt",
                    "b/cat.txt",
                    "c/dog.txt",
                    "a/egg.txt",
                    "fish.txt",
                    "a/goat.txt",
                ],
            ),
            (
                vec!["unicorn.txt"],
                vec!["zebra.txt"],
                vec![
                    ("a/bee.txt", "bee.txt"),
                    ("b/dog.txt", "c/dog.txt"),
                    ("c/egg.txt", "a/egg.txt"),
                    ("c/fish.txt", "fish.txt"),
                    ("c/goat.txt", "a/goat.txt"),
                ],
            ),
        )
    }

    #[test]
    fn diff_6() {
        assert_diff_paths(
            (
                vec!["a/ant.txt", "b/ant.txt", "c/ant.txt"],
                vec!["a/ant.txt", "a/b/ant.txt", "a/c/ant.txt"],
            ),
            (
                vec![],
                vec![],
                vec![("b/ant.txt", "a/b/ant.txt"), ("c/ant.txt", "a/c/ant.txt")],
            ),
        )
    }
}
