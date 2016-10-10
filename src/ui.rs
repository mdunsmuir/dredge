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
        // this poor tortured if statement exists to avoid borrow conflicts
        if {
            let mut fst = &self.fst;

            for name in self.stack.iter() {
                fst = fst.entry(name).unwrap();
            }

            if !fst.is_empty().unwrap() {

                self.listing = fst.list().unwrap();

                true
            } else {
                false
            }

        } { // this is the "then" block...
            let selected = self.selected_mut();
            if selected.is_none() {
                *selected = Some(0);
            }
        }

        self.listing.sort_by_key(|&(_, size, _)| size );
        self.listing.reverse();
    }

    pub fn event_loop(&mut self) {
        loop {
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

                _ => (),
            }
        }
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

        self.align_viewport();
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
                0, 0, rustbox::Style::empty(),
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
        let status_str = format!("Total Size: {}",
                                 Self::format_size(self.fst.size().unwrap()));

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
        let (ref name, size, is_dir) = *listing;

        // set colors depending on whether this line is selected
        let (front, back) = if selected {
            (rustbox::Color::Black, rustbox::Color::White)
        } else {
            (rustbox::Color::Default, rustbox::Color::Default)
        };

        // create the string for the size and directory indicator
        let size_str = Self::format_size(size);
        let size_and_dir_marker = if is_dir {
            format!("-> {:>}", size_str)
        } else {
            format!("   {:>}", size_str)
        };

        let size_str_x = self.rustbox.width() - size_and_dir_marker.len();

        // name on the left
        self.rustbox.print(
            0, y, rustbox::Style::empty(),
            front, back, name.to_str().unwrap()
        );

        // size on the right
        self.rustbox.print(
            size_str_x, y, rustbox::Style::empty(),
            front, back, &size_and_dir_marker
        );

        if selected {
            for col in name.to_str().unwrap().len()..size_str_x {
                self.rustbox.print_char(
                    col, y, rustbox::Style::empty(),
                    front, back, ' '
                )
            }
        }
    }

    fn format_size(size: u64) -> String {
        if size == 0 {
            return format!("{:>} {}", 0, 'B');
        }

        let (prefix, power) = match (size as f64).log(1000.0).floor() as i32 {
            0 => ('B', 0),
            1 => ('K', 1),
            2 => ('M', 2),
            3 => ('G', 3),
            x if x > 3 => ('T', 3),
            _ => ('B', 0),
        };

        format!("{:>10.1} {}", size as f64 / (1000.0 as f64).powi(power), prefix)
    }
}
