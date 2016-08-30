extern crate std;

use super::*;

use rustbox::Event::KeyEvent;
use rustbox::keyboard::Key::*;

const LINE_MAX_WIDTH: usize = 50;

pub struct UI<'a> {
    fst: FSTree,
    rustbox: &'a rustbox::RustBox,
    stack: Vec<std::ffi::OsString>,
    listing: Vec<Listing>,
    selected: Option<usize>,
    window_top: usize,
}

impl<'a> UI<'a> {

    pub fn new(rustbox: &'a rustbox::RustBox, fsts: FSTree) -> Self {
        let mut ui = UI {
            fst: fsts,
            rustbox: rustbox,
            stack: Vec::new(),
            listing: Vec::new(),
            selected: None,
            window_top: 0,
        };

        ui.load();
        ui
    }

    pub fn load(&mut self) {
        let mut fst = &self.fst;

        for name in self.stack.iter() {
            fst = fst.entry(name).unwrap();
        }

        if fst.is_empty().unwrap() {
            self.selected = None;
        } else {
            self.selected = Some(0);
            self.listing = fst.list().unwrap();
        }

        self.listing.sort_by_key(|&(_, size, _)| size );
        self.listing.reverse();
    }

    pub fn event_loop(&mut self) {
        loop {
            self.draw();

            match self.rustbox.poll_event(false) {
                Ok(KeyEvent(Char('q'))) => break,

                Ok(KeyEvent(Char('k'))) => {
                    let mut o_cur = std::mem::replace(&mut self.selected, None);

                    self.selected = o_cur.map(|cur| if cur > 0 {
                        cur - 1
                    } else { 
                        cur 
                    });
                },

                Ok(KeyEvent(Char('j'))) => {
                    let mut o_cur = std::mem::replace(&mut self.selected, None);

                    self.selected = o_cur.map(|cur|
                        std::cmp::min(self.listing.len() - 1, cur + 1)
                    );
                },

                Ok(KeyEvent(Char('l'))) => {
                    if let Some(pos) = self.selected {
                        let (name, is_dir) = {
                            let ref target = self.listing[pos];
                            (target.0.clone(), target.2)
                        };

                        if is_dir {
                            self.stack.push(name);
                            self.load();
                        }  
                    }
                }

                Ok(KeyEvent(Char('h'))) => {
                    if let Some(_) = self.stack.pop() {
                        self.load();
                    }
                },

                _ => (),
            }
        }
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
                    self.draw_line(i, i + self.window_top == i_selected, line);
                }
            }
        }

        self.rustbox.present();
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
