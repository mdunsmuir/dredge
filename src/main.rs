#[macro_use]
extern crate clap;
pub extern crate rustbox;

pub mod fstree2;
pub mod os;
pub mod ui;

pub use fstree2::*;
pub use ui::*;

#[derive(Debug)]
pub enum Error {
    RootIsNotDirectory,
    IO(std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

fn main() {
    let args = clap_app!(app =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: "A tool to inspect and clean up directories and files")
        (@arg PATH: +required "The root directory to inspect")
    ).get_matches();

    let path = std::path::PathBuf::from(args.value_of("PATH").unwrap());

    println!("loading...");
    let mut fsts = FSTree::from_dir(&path).unwrap();

    let mut opts = rustbox::InitOptions::default();
    opts.buffer_stderr = true;
    let rustbox = rustbox::RustBox::init(opts).unwrap();

    let mut ui = UI::new(&rustbox, fsts);
    ui.event_loop();
}
