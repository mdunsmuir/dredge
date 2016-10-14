extern crate std;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::fs;
use super::os;

pub type Listing = (OsString, u64, bool, Option<OsString>);

pub struct Contents(BTreeMap<OsString, FSTree>);

pub enum FSTree {
    Root {
        contents: Contents,
        path: PathBuf,
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

    Symlink {
        path: PathBuf,
        metadata: fs::Metadata,
        target: PathBuf,
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

    fn get_map_mut(&mut self) -> &mut BTreeMap<OsString, FSTree> {
        let Contents(ref mut map) = *self;
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
    fst_accessor!(path, std::path::PathBuf, Root, Dir, File, Symlink);
    fst_accessor!(metadata, std::fs::Metadata, Dir, File, Symlink);
    fst_accessor!(total_size, u64, Root, Dir);

    fst_accessor!(mut: contents_mut, contents, Contents, Root, Dir);
    fst_accessor!(mut: total_size_mut, total_size, u64, Root, Dir);

    variant_checker!(is_root, Root);
    variant_checker!(is_dir, Dir);
    variant_checker!(is_file, File);
    variant_checker!(is_symlink, Symlink);
    variant_checker!(is_bad, Bad);

    /// Get the size of this object in bytes. `Bad` objects don't have any
    /// reportable size, hence the `Option`.
    pub fn size(&self) -> Option<u64> {
        self.total_size().cloned().or_else(||
            if self.is_file() || self.is_symlink() {
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

            } else if md.file_type().is_symlink() {
                fs::read_link(entry.path()).map(|target|
                    FSTree::Symlink {
                        path: entry.path().clone(),
                        metadata: md,
                        target: target,
                    }
                ).ok()

            } else { // not sure if this can even happen, but...
                None
            }
        }).unwrap_or(FSTree::Bad)
    }

    pub fn from_dir<P: AsRef<Path>>(path: P) -> Option<Self> {
        let path_buf = path.as_ref().to_path_buf();

        Contents::from_path(path).map(|contents| {
            let size = contents.size();
            FSTree::Root {
                contents: contents,
                path: path_buf,
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
                    fst.is_dir(),

                    if let FSTree::Symlink { ref target, .. } = *fst {
                        Some(target.as_os_str().to_os_string())
                    } else {
                        None
                    },
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

    pub fn entries(&self, names: &[OsString]) -> Option<&FSTree> {
        let mut fst = Some(self);

        for name in names {
            let cur = fst.take();
            fst = cur.map(|fst| fst.entry(name) )
                .unwrap_or_else(|| return None )
        }

        fst
    }

    pub fn entry_mut(&mut self, name: &OsString) -> Option<&mut FSTree> {
        self.contents_mut().and_then(|n_contents| {
            let contents = n_contents.get_map_mut();
            contents.get_mut(name)
        })
    }

    pub fn is_empty(&self) -> Option<bool> {
        self.contents().map(|n_contents|
            n_contents.get_map().is_empty()
        )
    }

    pub fn delete_path(&mut self, names: &[OsString]) -> Option<u64> {
        if names.is_empty() {
            panic!("cannot delete empty path");

        } else {
            let name = names.first().unwrap();
            let others = &names[1..];

            if names.len() == 1 { // delete from this level
                let o_deleted_size = self.contents()
                    .map(|cs| cs.get_map() )
                    .and_then(|map| {map.get(name)})
                    .and_then(|fst| {

                    let fst_size = fst.size();
                    fst.delete().ok().and_then(|_| fst_size )
                });

                o_deleted_size.map(|size| {
                    *self.total_size_mut().unwrap() -= size;

                    self.contents_mut().map(|cs| cs.get_map_mut() ).and_then(|map| {
                        map.remove(name)
                    });

                    *self.total_size().unwrap()
                })

            } else { // go deeper to delete
                let mut cur_size = None;

                // recursive call to delete
                let new_size = self.entry_mut(name) .and_then(|fst| {
                    cur_size = fst.size();
                    fst.delete_path(others)
                });

                // now that we have the new size, update this node and pass
                // *its* new size on down the line
                new_size.map(|new_size| {
                    let size_delta = cur_size.unwrap() - new_size;
                    *self.total_size_mut().unwrap() -= size_delta;
                    *self.total_size().unwrap()
                })
            }
        }
    }

    fn delete(&self) -> std::io::Result<()> {
        match *self {
            FSTree::Dir { ref path, .. } => fs::remove_dir_all(path),
            FSTree::File { ref path, .. } => fs::remove_file(path),
            FSTree::Symlink { ref path, .. } => fs::remove_file(path),
            _ => Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Could not delete this entry"
                )
            ),
        }
    }
}
