#[macro_use]
extern crate clap;
extern crate rustbox;

use std::collections::BTreeMap;
use std::os::linux::fs::MetadataExt;

/// I'm still totally confused about where this comes from and couldn't find
/// an API to grab it... maybe in system header files?
///
/// ST_BLKSIZE returns the *IO* block size, which is 4096
const DEVICE_BLOCKSIZE: u64 = 512;

#[derive(Debug)]
enum Error {
    RootIsNotDirectory,
    IO(std::io::Error),
}

type Result<T> = std::result::Result<T, Error>;
type Contents = BTreeMap<std::ffi::OsString, FSTree>;

enum FSTree {
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

struct UIState {
    stack: Vec<std::ffi::OsString>,
    selected: usize,
    window_top: usize,
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

    fn is_bad(&self) -> bool {
        match *self {
            FSTree::Bad => true,
            _ => false,
        }
    }

    fn size(&self) -> u64 {
        match *self {
            FSTree::Dir { total_size, .. } => total_size,
            FSTree::File { ref metadata, .. } =>
                metadata.st_blocks() * DEVICE_BLOCKSIZE,
            FSTree::Bad => 0,
        }
    }

    fn from_root(path: &std::path::PathBuf) -> Result<Contents> {
        let md = match std::fs::metadata(path) {
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

    fn new(entry: std::fs::DirEntry) -> FSTree {
        entry.metadata().and_then(|md|
            if md.is_dir() {
                std::fs::read_dir(entry.path()).and_then::<Contents, _>(|entries|
                    Ok(map_entries!(entries))

                ).and_then(|contents| {
                    let num_files = contents.len();
                    let my_size = md.st_blocks() * DEVICE_BLOCKSIZE;
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

impl UIState {

    fn new() -> Self {
        UIState {
            stack: Vec::new(),
            selected: 0,
            window_top: 0,
        }
    }
}

fn main() {
    let args = clap_app!(app =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: "A tool to inspect and clean up directories and files")
        (@arg PATH: +required "The root directory to inspect")
    ).get_matches();

    let path = std::path::PathBuf::from(args.value_of("PATH").unwrap());

    println!("loading...");
    let mut fsts = FSTree::from_root(&path).unwrap();
    let mut ui_state = UIState::new();
    let rustbox = rustbox::RustBox::init(rustbox::InitOptions::default()).unwrap();

    loop {
        draw(&mut fsts, &rustbox);

        match rustbox.poll_event(false) {
            Ok(rustbox::Event::KeyEvent(rustbox::keyboard::Key::Char('q'))) => break,
            _ => (),
        }
    }
}

fn draw(fsts: &mut Contents, rustbox: &rustbox::RustBox) {
    rustbox.clear();
    rustbox.present();
}
