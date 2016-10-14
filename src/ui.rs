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

extern crate std;

use super::*;

use rustbox::Event::KeyEvent;
use rustbox::keyboard::Key::*;

pub struct UI<'a> {
    fst: FSTree,
    rustbox: &'a rustbox::RustBox,
    stack: Vec<std::ffi::OsString>,
    listing: Vec<Listing>,
    selected: Vec<Option<usize>>,
    window_top: usize,
}

impl<'a> UI<'a> {

    pub fn new(rustbox: &'a rustbox::RustBox, fsts: FSTree) -> Self {
        let mut ui = UI {
            fst: fsts,
            rustbox: rustbox,
            stack: Vec::new(),
            listing: Vec::new(),
            selected: vec![None],
            window_top: 0,
        };

        ui.load();
        ui
    }

    pub fn load(&mut self) {
        self.listing = self.fst.entries(self.stack.as_slice())
                               .and_then(|fst| fst.list() )
                               .unwrap();

        self.listing.sort_by_key(|&(_, size, _, _)| size );
        self.listing.reverse();

        if !self.listing.is_empty() { // if there are items to show
            let n_listings = self.listing.len();
            let selected = self.selected_mut();

            // if no previous selection (i.e. we came from above) then we
            // start with the first item selected (otherwise remember previous)
            if selected.is_none() {
                *selected = Some(0);
            }

            // we might be off the end of the list... if so, bump up
            selected.take().map(|pos|
                *selected = Some(std::cmp::min(n_listings - 1, pos))
            );

        } else { // if no items to show, make sure
                 // we don't think anything is selected
            *self.selected_mut() = None;
        }
    }

    pub fn event_loop(&mut self) {
        loop {
            self.align_viewport();
            self.draw();

            match self.rustbox.poll_event(false) {
                Ok(KeyEvent(Char('q'))) => break,

                Ok(KeyEvent(Char('k'))) => self.scroll(-1),
                Ok(KeyEvent(Char('j'))) => self.scroll(1),

                Ok(KeyEvent(PageUp))  => self.scroll(-(self.rustbox.height() as i32)),
                Ok(KeyEvent(PageDown))  => self.scroll(self.rustbox.height() as i32),

                Ok(KeyEvent(Char('l'))) => {
                    if let &Some(pos) = self.selected() {
                        let (name, is_dir) = {
                            let ref target = self.listing[pos];
                            (target.0.clone(), target.2)
                        };

                        if is_dir {
                            self.stack.push(name);
                            self.selected.push(None);
                            self.load();
                        }
                    }
                }

                Ok(KeyEvent(Char('h'))) => {
                    if let Some(_) = self.stack.pop() {
                        self.selected.pop();
                        self.load();
                    }
                },

                Ok(KeyEvent(Char('d'))) => self.delete(),

                _ => (),
            }
        }
    }

    fn delete(&mut self) {
        // get the current selection position, or no-op if nothing is
        // selected (indicative of empty dir)
        let pos = match *self.selected() {
            None => return,
            Some(pos) => pos,
        };

        // clear screen and show the prompt
        self.rustbox.clear();
        self.draw_status_bar(0);

        self.stack.push(self.listing[pos].0.clone());

        let path = {
            let fst = self.fst.entries(self.stack.as_slice()).unwrap();
            if fst.is_bad() {
                return
            } else {
                fst.path().unwrap().clone()
            }
        };

        let prompt = format!(
            "Really delete {} ? (y/N)",
            path.to_str().unwrap()
        );

        self.rustbox.print(
            0, 1, rustbox::Style::empty(),
            rustbox::Color::White,
            rustbox::Color::Default,
            &prompt
        );

        self.rustbox.present();

        match self.rustbox.poll_event(false) {
            Ok(KeyEvent(Char('y'))) => {
                self.rustbox.clear();
                self.draw_status_bar(0);

                self.rustbox.print(
                    0, 1, rustbox::Style::empty(),
                    rustbox::Color::White,
                    rustbox::Color::Default,
                    "deleting... this may take a little while"
                );

                self.rustbox.present();
                self.fst.delete_path(self.stack.as_slice());
                ()
            },

            _ => (),
        }

        self.stack.pop();
        self.load();
    }

    fn selected(&self) -> &Option<usize> {
        // unwrapping in these methods should be fine because we well always
        // have at least one level pushed to the line selection stack
        // (even if the value is `None`)
        self.selected.last().unwrap()
    }

    fn selected_mut(&mut self) -> &mut Option<usize> {
        self.selected.last_mut().unwrap()
    }

    fn scroll(&mut self, distance: i32) {
        let listing_len = self.listing.len();

        if let Some(selected) = self.selected_mut().as_mut() {
            let new_selected =
                std::cmp::max(
                    0,
                    std::cmp::min(
                        listing_len as i32 - 1,
                        *selected as i32 + distance
                    )
                ) as usize;

            *selected = new_selected;
        }
    }

    // if the selected line has gone off the screen, we need to re-align the
    // viewport to make it visible again.
    fn align_viewport(&mut self) {
        let height = self.rustbox.height() - 1; // minus one for status bar

        if let Some(&selected) = self.selected().as_ref() {
            if selected < self.window_top {
                self.window_top = selected;

            } else if selected >= self.window_top + height {
                self.window_top = selected - height + 1;
            }
        }
    }

    fn draw(&self) {
        self.rustbox.clear();

        match self.selected().as_ref() {
            None => self.rustbox.print(
                0, 1, rustbox::Style::empty(),
                rustbox::Color::White,
                rustbox::Color::Default,
                "<no files>"
            ),

            Some(&i_selected) => {
                // subtract one so the status bar fits
                let height = self.rustbox.height() - 1;
                let last_index = // actually last index + 1
                    std::cmp::min((self.window_top + height), self.listing.len());
                let to_display =
                    &self.listing[self.window_top..last_index];

                for (i, line) in to_display.iter().enumerate() {
                    self.draw_line(i + 1, i + self.window_top == i_selected, line);
                }
            }
        }

        self.draw_status_bar(0);
        self.rustbox.present();
    }

    fn draw_status_bar(&self, y: usize) {
        let status_str = {
            let root_path = self.fst.path().unwrap();
            let root_size = self.fst.size().unwrap();

            let cur_fst = self.fst.entries(self.stack.as_slice()).unwrap();
            let cur_size = cur_fst.size().unwrap();
            let cur_path = cur_fst.path().unwrap();

            // if we're at the root, there's no path worth showing
            if self.stack.is_empty() {
                format!("{} : {}",
                    root_path.to_str().unwrap(),
                    Self::format_size(root_size),
                )

            } else {
                format!(
                    "{} : {} | {} : {}",
                    root_path.to_str().unwrap(),
                    Self::format_size(root_size),
                    cur_path.to_str().unwrap(),
                    Self::format_size(cur_size),
                )
            }
        };

        self.rustbox.print(
            0, y, rustbox::Style::empty(),
            rustbox::Color::Default, rustbox::Color::Red,
            &status_str
        );

        for col in status_str.len()..self.rustbox.width() {
            self.rustbox.print_char(
                col, y, rustbox::Style::empty(),
                rustbox::Color::Default, rustbox::Color::Red, ' '
            )
        }
    }

    fn draw_line(&self, y: usize, selected: bool, listing: &Listing) {
        // set colors depending on whether this line is selected
        let (front, back) = if selected {
            (rustbox::Color::Black, rustbox::Color::White)
        } else {
            (rustbox::Color::Default, rustbox::Color::Default)
        };

        let (name_part, size_and_dir_part) = self.format_listing(listing);
        let size_str_x = self.rustbox.width() - size_and_dir_part.len();

        // name on the right
        self.rustbox.print(
            0, y, rustbox::Style::empty(),
            front, back, &name_part
        );

        // size on the right
        self.rustbox.print(
            size_str_x, y, rustbox::Style::empty(),
            front, back, &size_and_dir_part
        );

        // and fill in the highlighted line if needed
        if selected {
            for col in name_part.len()..size_str_x {
                self.rustbox.print_char(
                    col, y, rustbox::Style::empty(),
                    front, back, ' '
                )
            }
        }
    }

    fn format_listing(&self, listing: &Listing) -> (String, String) {
        let (ref name, size, is_dir, ref symlink_target) = *listing;

        // create the string for the size and directory indicator
        let size_str = Self::format_size(size);
        let size_and_dir_part = if is_dir {
            format!("-> {:>10}", size_str)
        } else {
            format!("   {:>10}", size_str)
        };

        let name_part = if let Some(ref target) = *symlink_target {
            format!(
                "{} -> {}",
                name.to_str().unwrap(),
                target.to_str().unwrap()
            )
        } else {
            String::from(name.to_str().unwrap())
        };

        (name_part, size_and_dir_part)
    }

    fn format_size(size: u64) -> String {
        if size == 0 {
            return format!("{:>} {}", 0, 'B');
        }

        let (prefix, power) = match (size as f64).log(1024.0).floor() as i32 {
            0 => ("B", 0),
            1 => ("KiB", 1),
            2 => ("MiB", 2),
            3 => ("GiB", 3),
            x if x > 3 => ("TiB", 4),
            _ => ("B", 0),
        };

        format!("{:>.1} {}", size as f64 / (1024.0 as f64).powi(power), prefix)
    }
}
