extern crate std;

use std::collections::BTreeMap;
use super::*;

pub type Contents = BTreeMap<std::ffi::OsString, FSTree>;

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

    pub fn new(entry: std::fs::DirEntry) -> FSTree {
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
