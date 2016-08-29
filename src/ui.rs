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
