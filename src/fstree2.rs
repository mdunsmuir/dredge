extern crate std;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::fs;
use super::os;

pub type Listing = (OsString, u64, bool);

pub struct Contents(BTreeMap<OsString, FSTree>);

pub enum FSTree {
    Root {
        contents: Contents,
        total_size: u64,
    },

    Dir {
        contents: Contents,
        path: PathBuf,
        metadata: fs::Metadata,
        total_size: u64,
    },

    File {
        path: PathBuf,
        metadata: fs::Metadata,
    },

    Bad,
}

impl Contents {

    fn from_path<P: AsRef<Path>>(path: P) -> Option<Contents> {
        fs::read_dir(path).map(|r_entries| // map over the directory entries
            Contents(r_entries.filter_map(|r_entry| // they are in Results
                r_entry.map(|entry|
                    (
                        entry.file_name(),
                        FSTree::from_dir_entry(entry),
                    )
                ).ok()
            ).collect())
        ).ok()
    }

    fn size(&self) -> u64 {
        self.get_map().values()
           .map(|fst| fst.size().unwrap_or(0) )
           .fold(0, |a, b| a + b )
    }

    fn get_map(&self) -> &BTreeMap<OsString, FSTree> {
        let Contents(ref map) = *self;
        map
    }
}

macro_rules! fst_accessor {
    ( $name:ident,
      $return_type:ty, 
      $( $variant:ident ),+
    ) => {
        pub fn $name(&self) -> Option<&$return_type> {
            match *self {
                $(
                    FSTree::$variant { ref $name, .. } => Some($name),
                )+
                _ => None,
            }
        }
    };

    (mut: $name:ident,
     $field:ident,
     $return_type:ty,
     $( $variant:ident ),*
    ) => {
        fn $name(&mut self) -> Option<&mut $return_type> {
            match *self {
                $(
                    FSTree::$variant { ref mut $field, .. } => Some($field),
                )+
                _ => None,
            }
        }
    }
}

macro_rules! variant_checker {
    ( $name:ident, $variant:ident ) => {
        pub fn $name(&self) -> bool {
            if let FSTree::$variant { .. } = *self {
                true
            } else {
                false
            }
        }
    }
}

impl FSTree {

    fst_accessor!(contents, Contents, Root, Dir);
    fst_accessor!(path, std::path::PathBuf, Dir, File);
    fst_accessor!(metadata, std::fs::Metadata, Dir, File);
    fst_accessor!(total_size, u64, Root, Dir);

    fst_accessor!(mut: contents_mut, contents, Contents, Root, Dir);
    fst_accessor!(mut: total_size_mut, total_size, u64, Root, Dir);

    variant_checker!(is_root, Root);
    variant_checker!(is_dir, Dir);
    variant_checker!(is_file, File);
    variant_checker!(is_bad, Bad);

    /// Get the size of this object in bytes. `Bad` objects don't have any
    /// reportable size, hence the `Option`.
    pub fn size(&self) -> Option<u64> {
        self.total_size().cloned().or_else(||
            if self.is_file() {
                self.metadata().map(|md| os::size(md) )
            } else {
                None
            }
        )
    }

    fn from_dir_entry(entry: fs::DirEntry) -> Self {
        entry.metadata().ok().and_then(|md| {
            if md.is_dir() {
                Contents::from_path(entry.path()).map(|contents| {
                    let total_size = contents.size();

                    FSTree::Dir {
                        contents: contents,
                        path: entry.path().clone(),
                        metadata: md,
                        total_size: total_size,
                    }
                })

            } else if md.is_file() {
                Some(FSTree::File {
                    path: entry.path().clone(),
                    metadata: md,
                })
            } else {
                None
            }
        }).unwrap_or(FSTree::Bad)
    }

    pub fn from_dir<P: AsRef<Path>>(path: P) -> Option<Self> {
        Contents::from_path(path).map(|contents| {
            let size = contents.size();
            FSTree::Root {
                contents: contents,
                total_size: size,
            }
        })
    }

    pub fn list(&self) -> Option<Vec<Listing>> {
        self.contents().map(|&Contents(ref contents)|
            contents.iter().map(|(name, fst)|
                (
                    name.clone(),
                    fst.size().unwrap_or(0),
                    fst.is_dir()
                )
            ).collect()
        )
    }

    pub fn entry(&self, name: &OsString) -> Option<&FSTree> {
        self.contents().and_then(|n_contents| {
            let contents = n_contents.get_map();
            contents.get(name)
        })
    }

    pub fn is_empty(&self) -> Option<bool> {
        self.contents().map(|n_contents|
            n_contents.get_map().is_empty()
        )
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn it_works() {
        assert!(true);
    }
}
