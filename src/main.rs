#![feature(type_alias_impl_trait)]
#![feature(associated_type_defaults)]
#![feature(generic_associated_types)]
use std::vec;

use git2::{Repository, StatusOptions};

#[derive(Debug)]
enum ChangeFiles {
    NotStagedModified(String),
    NotStagedRenamed(String, String),
}

//
trait Status {
    fn untracked_files(&self) -> Option<Vec<&str>>;
    fn not_staged_modify_files(&self) -> Option<Vec<ChangeFiles>>;
    fn staged_files(&self) -> Option<Vec<&str>>;
}

impl Status for Repository {
    fn untracked_files(&self) -> Option<Vec<&str>> {
        None
    }

    fn not_staged_modify_files(&self) -> Option<Vec<ChangeFiles>> {
        let mut options = StatusOptions::new();
        let statuses = match self.statuses(Some(&mut options)) {
            Ok(statuses) => statuses,
            Err(err) => panic!("Get status failed, {}", err),
        };
        let mut vec = vec![];
        for status in statuses.iter() {
            let old_path = status.index_to_workdir().unwrap().old_file().path();
            let new_path = status.index_to_workdir().unwrap().new_file().path();
            match (old_path, new_path) {
                (Some(old), Some(new)) if old != new => vec.push(ChangeFiles::NotStagedRenamed(
                    old.to_str().unwrap().into(),
                    new.to_str().unwrap().into(),
                )),
                (old, new) => {
                    vec.push(ChangeFiles::NotStagedModified(
                        old.or(new).unwrap().to_str().unwrap().into(),
                    ));
                }
            }
        }
        Some(vec)
    }

    fn staged_files(&self) -> Option<Vec<&str>> {
        None
    }
}

fn main() {
    let repo = match Repository::open("/home/locez/rust/git-ws/") {
        Ok(repo) => repo,
        Err(err) => panic!("open error, {}", err),
    };
    let mut options = StatusOptions::new();
    options.include_untracked(true);
    let r = repo.not_staged_modify_files();
    println!("{:?}", r.unwrap())
}
