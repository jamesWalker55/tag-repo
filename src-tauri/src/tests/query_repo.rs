use crate::query::to_sql;
use crate::repo::{Repo, TestRepo};

mod PresetRepo {
    use super::*;
    use crate::repo::Repo;
    use serde_json::{from_str, Value};

    fn insert_from_testrepo_json(repo: &mut Repo, data: &str) {
        let data: Value = from_str(data).unwrap();
        let rows = data.as_array().unwrap();

        repo.insert_items(rows.iter().map(|row| {
            let row = row.as_array().unwrap();
            (
                row.get(0).unwrap().as_str().unwrap(),
                row.get(1).unwrap().as_str().unwrap(),
            )
        }))
        .unwrap();
    }

    pub(crate) fn drum_collection() -> TestRepo {
        const data: &str = include_str!("drum_collection.testrepo.json");
        let mut tr = TestRepo::new();
        insert_from_testrepo_json(&mut tr.repo, data);
        tr
    }
}

fn assert_query(repo: Repo, query: &str, paths: Vec<&str>) {
    let items = repo.query_items(query).unwrap();
    let item_paths: Vec<&str> = items.iter().map(|i| i.path.as_ref()).collect();
    assert_eq!(item_paths, paths)
}

#[test]
fn query_1() {
    assert_query(
        PresetRepo::drum_collection().repo,
        "drum (snare | clap)",
        vec![
            "Drum Collection/Clap/Clap 1.wav",
            "Drum Collection/Clap/Clap 2.wav",
            "Drum Collection/Clap/Clap 3.wav",
            "Drum Collection/Snare/Snare 1.wav",
            "Drum Collection/Snare/Snare 2.wav",
            "Drum Collection/Snare/Snare 3.wav",
        ],
    );
}
