#[macro_use]
extern crate clap;
extern crate rustbox;

use std::collections::BTreeMap;

mod os {

    use std::fs::Metadata;

    /// I'm still totally confused about where this comes from and couldn't find
    /// an API to grab it... maybe in system header files?
    ///
    /// ST_BLKSIZE returns the *IO* block size, which is 4096
    const DEVICE_BLOCKSIZE: u64 = 512;

    // linux

    #[cfg(target_family = "linux")]
    use std::os::linux::fs::MetadataExt;

    #[cfg(target_family = "linux")]
    pub fn size(metadata: &Metadata) -> u64 {
        metadata.st_blocks() * DEVICE_BLOCKSIZE
    }

    // unix

    #[cfg(target_family = "unix")]
    use std::os::unix::fs::MetadataExt;

    #[cfg(target_family = "unix")]
    pub fn size(metadata: &Metadata) -> u64 {
        metadata.blocks() * DEVICE_BLOCKSIZE
    }
}

const LINE_MAX_WIDTH: usize = 50;

#[derive(Debug)]
enum Error {
    RootIsNotDirectory,
    IO(std::io::Error),
}

type Result<T> = std::result::Result<T, Error>;
type Contents = BTreeMap<std::ffi::OsString, FSTree>;
type Listing = (std::ffi::OsString, u64, bool);

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

struct UI<'a> {
    rustbox: &'a rustbox::RustBox,
    stack: Vec<std::ffi::OsString>,
    listing: Vec<Listing>,
    selected: Option<usize>,
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

    fn is_dir(&self) -> bool {
        match *self {
            FSTree::Dir { .. } => true,
            _ => false,
        }
    }

    fn size(&self) -> u64 {
        match *self {
            FSTree::Dir { total_size, .. } => total_size,
            FSTree::File { ref metadata, .. } => os::size(metadata),
            FSTree::Bad => 0,
        }
    }

    fn from_root(path: &std::path::PathBuf) -> Result<Contents> {
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

impl<'a> UI<'a> {

    fn new(rustbox: &'a rustbox::RustBox, fsts: &Contents) -> Self {
        let mut ui = UI {
            rustbox: rustbox,
            stack: Vec::new(),
            listing: Vec::new(),
            selected: None,
            window_top: 0,
        };

        ui.load(fsts);
        ui
    }

    fn load(&mut self, mut fsts: &Contents) {
        self.listing.clear();

        for name in self.stack.iter() {
            fsts = match *fsts.get(name).unwrap() {
                FSTree::Dir { ref contents, .. } => contents,
                _ => panic!("expected Dir"),
            };
        }

        if fsts.is_empty() {
            self.selected = None;
        } else {
            self.selected = Some(0);

            for (name, fst) in fsts {
                self.listing.push((name.clone(), fst.size(), fst.is_dir()));
            }
        }

        self.listing.sort_by_key(|&(_, size, _)| size );
        self.listing.reverse();
    }

    fn draw(&self) {
        self.rustbox.clear();

        match self.selected {
            None => self.rustbox.print(
                0, 0, rustbox::Style::empty(),
                rustbox::Color::White,
                rustbox::Color::Default,
                "<no files>"
            ),

            Some(i_selected) => {
                let height = self.rustbox.height();
                let width = self.rustbox.width();
                let last_index = // actually last index + 1
                    std::cmp::min((self.window_top + height), self.listing.len());
                let to_display = 
                    &self.listing[self.window_top..last_index];

                for (i, line) in to_display.iter().enumerate() {
                    let (front, back) = if i + self.window_top  == i_selected {
                        (rustbox::Color::Black, rustbox::Color::White)
                    } else {
                        (rustbox::Color::Default, rustbox::Color::Default)
                    };

                    self.rustbox.print(
                        0, i, rustbox::Style::empty(),
                        front, back, &self.format_line(line)
                    );
                }
            }
        }

        self.rustbox.present();
    }

    fn format_line(&self, line: &Listing) -> String {
        let width = self.rustbox.width();
        let (ref name, size, is_dir) = *line;
        let size_mb = size as f64 / 1000000.0;

        if is_dir {
            format!("{:<28}->{:>10.1} M", name.to_str().unwrap(), size_mb)
        } else {
            format!("{:<30}{:>10.1} M", name.to_str().unwrap(), size_mb)
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

    let mut opts = rustbox::InitOptions::default();
    opts.buffer_stderr = true;
    let rustbox = rustbox::RustBox::init(opts).unwrap();

    let mut ui_state = UI::new(&rustbox, &fsts);

    loop {
        ui_state.draw();

        match rustbox.poll_event(false) {
            Ok(rustbox::Event::KeyEvent(rustbox::keyboard::Key::Char('q'))) => break,

            Ok(rustbox::Event::KeyEvent(rustbox::keyboard::Key::Char('k'))) => {
                let mut o_cur = std::mem::replace(&mut ui_state.selected, None);
                ui_state.selected = o_cur.map(|cur| if cur > 0 { cur - 1 } else { cur } );
            },

            Ok(rustbox::Event::KeyEvent(rustbox::keyboard::Key::Char('j'))) => {
                let mut o_cur = std::mem::replace(&mut ui_state.selected, None);

                ui_state.selected = o_cur.map(|cur|
                    std::cmp::min(ui_state.listing.len() - 1, cur + 1)
                );
            },

            Ok(rustbox::Event::KeyEvent(rustbox::keyboard::Key::Char('l'))) => {
                match ui_state.selected {
                    None => (),
                    Some(pos) => {
                        let (name, is_dir) = {
                            let ref target = ui_state.listing[pos];
                            (target.0.clone(), target.2)
                        };

                        if is_dir {
                            ui_state.stack.push(name);
                            ui_state.load(&fsts);
                        }  
                    }
                }
            }

            Ok(rustbox::Event::KeyEvent(rustbox::keyboard::Key::Char('h'))) => {
                if let Some(_) = ui_state.stack.pop() {
                    ui_state.load(&fsts);
                }
            },

            _ => (),
        }
    }
}

