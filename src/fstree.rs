extern crate std;

use std::ffi::OsString;
use std::collections::BTreeMap;
use super::*;

pub type Contents = BTreeMap<OsString, FSTree>;

pub enum FSTree {
    Dir {
        path: std::path::PathBuf,
        metadata: std::fs::Metadata,
        total_size: u64,
        num_files: usize,
        contents: Contents,
    },

    File {
        path: std::path::PathBuf,
        metadata: std::fs::Metadata,
    },

    Bad
}

macro_rules! map_entries {
    ( $entries:ident ) => {
        $entries.map(|r_entry| 
            r_entry.map(|entry| (entry.file_name(), Self::new(entry)) )
        ).filter_map(|r_fst| match r_fst {
            Ok(fst) => Some(fst),
            Err(_) => None,
        }).collect()
    }
}

impl FSTree {

    pub fn is_bad(&self) -> bool {
        match *self {
            FSTree::Bad => true,
            _ => false,
        }
    }

    pub fn is_dir(&self) -> bool {
        match *self {
            FSTree::Dir { .. } => true,
            _ => false,
        }
    }

    pub fn path(&self) -> Option<&std::path::PathBuf> {
        match *self {
            FSTree::Dir { ref path, .. } => Some(path),
            FSTree::File { ref path, .. } => Some(path),
            FSTree::Bad => None,
        }
    }

    pub fn size(&self) -> u64 {
        match *self {
            FSTree::Dir { total_size, .. } => total_size,
            FSTree::File { ref metadata, .. } => os::size(metadata),
            FSTree::Bad => 0,
        }
    }

    pub fn from_root(path: &std::path::PathBuf) -> Result<Contents> {
        match std::fs::metadata(path) {
            Ok(md) => if md.is_dir() {
                md
            } else {
                return Err(Error::RootIsNotDirectory)
            },

            Err(e) => return Err(Error::IO(e)),
        };

        std::fs::read_dir(path).map(|entries| map_entries!(entries) )
                               .map_err(|e| Error::IO(e) )
    }

    pub fn delete(contents: &mut Contents, path: &[OsString]) -> Option<u64> {
        if path.is_empty() {
            panic!("cannot delete empty path")

        } else if path.len() == 1 {
            let name = path.first().unwrap();

            let info = contents.get(name)
                               .and_then(|fst| 
                fst.path().map(|p| (p.clone(), fst.size()) ) 
            );

            if let Some((pb, size)) = info {
                std::fs::remove_file(pb).ok().map(|_| {
                    contents.remove(name);
                    size
                })

            } else {
                None
            }

        } else {
            path.first()
                .and_then(|name| contents.get_mut(name) )
                .and_then(|fst| fst._delete(&path[1..]) )
        }
    }

    fn _delete(&mut self, path: &[OsString]) -> Option<u64> {
        if path.is_empty() {
            panic!("cannot delete empty path");

        } else if path.len() == 1 {
            let name = path.first().unwrap();

            let mut contents = match *self {
                FSTree::Dir { ref mut contents, .. } => contents,
                _ => panic!("expected Dir"),
            };

            let info = contents.get(name)
                               .and_then(|fst| 
                fst.path().map(|p| (p.clone(), fst.size()) ) 
            );

            if let Some((pb, size)) = info {
                std::fs::remove_file(pb).ok().map(|_| {
                    contents.remove(name);
                    size
                })

            } else {
                None
            }

        } else {
            let name = path.first().unwrap();
            let (contents, total_size) = match *self {
                FSTree::Dir { ref mut contents, ref mut total_size, .. } =>
                    (contents, total_size),
                _ => panic!("expected Dir"),
            };

            match contents.get_mut(name) {
                Some(fst) =>
                    fst._delete(&path[1..]).map(|size_deleted| {
                        *total_size -= size_deleted;
                        size_deleted
                    }),
                _ => panic!("expected Some"),
            }
        }
    }

    fn new(entry: std::fs::DirEntry) -> FSTree {
        entry.metadata().and_then(|md|
            if md.is_dir() {
                std::fs::read_dir(entry.path()).and_then::<Contents, _>(|entries|
                    Ok(map_entries!(entries))

                ).and_then(|contents| {
                    let num_files = contents.len();
                    let my_size = os::size(&md);
                    let total_size = 
                        contents.values()
                                .fold(0, |cum, fst| fst.size() + cum) + my_size;

                    Ok(FSTree::Dir {
                        path: entry.path(),
                        metadata: md,
                        total_size: total_size,
                        num_files: num_files,
                        contents: contents,
                    })
                })

            } else if md.is_file() {
                Ok(FSTree::File {
                    path: entry.path(),
                    metadata: md,
                })

            } else {
                Ok(FSTree::Bad)
            }
        ).unwrap_or(FSTree::Bad)
    }
}
