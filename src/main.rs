// Copyright (C) 2016  Michael Dunsmuir
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

#[macro_use]
extern crate clap;
pub extern crate rustbox;

pub mod fstree;
pub mod os;
pub mod ui;

pub use fstree::*;
pub use ui::*;

fn main() {
    let args = clap_app!(app =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: "A tool to inspect and clean up directories and files")
        (@arg PATH: +required "The root directory to inspect")
    ).get_matches();

    let path = std::path::PathBuf::from(args.value_of("PATH").unwrap());

    println!("loading...");
    let fsts = FSTree::from_dir(&path).unwrap();

    let mut opts = rustbox::InitOptions::default();
    opts.buffer_stderr = true;
    let rustbox = rustbox::RustBox::init(opts).unwrap();

    let mut ui = UI::new(&rustbox, fsts);
    ui.event_loop();
}
