extern crate std;

use super::*;

const LINE_MAX_WIDTH: usize = 50;

type Listing = (std::ffi::OsString, u64, bool);

pub struct UI<'a> {
    rustbox: &'a rustbox::RustBox,
    stack: Vec<std::ffi::OsString>,
    listing: Vec<Listing>,
    selected: Option<usize>,
    window_top: usize,
}

impl<'a> UI<'a> {

    pub fn new(rustbox: &'a rustbox::RustBox, fsts: &Contents) -> Self {
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

    pub fn load(&mut self, mut fsts: &Contents) {
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

    pub fn event_loop(&mut self, fsts: &mut Contents) {
        loop {
            self.draw();

            match self.rustbox.poll_event(false) {
                Ok(rustbox::Event::KeyEvent(rustbox::keyboard::Key::Char('q'))) => break,

                Ok(rustbox::Event::KeyEvent(rustbox::keyboard::Key::Char('k'))) => {
                    let mut o_cur = std::mem::replace(&mut self.selected, None);

                    self.selected = o_cur.map(|cur| if cur > 0 {
                        cur - 1
                    } else { 
                        cur 
                    });
                },

                Ok(rustbox::Event::KeyEvent(rustbox::keyboard::Key::Char('j'))) => {
                    let mut o_cur = std::mem::replace(&mut self.selected, None);

                    self.selected = o_cur.map(|cur|
                        std::cmp::min(self.listing.len() - 1, cur + 1)
                    );
                },

                Ok(rustbox::Event::KeyEvent(rustbox::keyboard::Key::Char('l'))) => {
                    match self.selected {
                        None => (),
                        Some(pos) => {
                            let (name, is_dir) = {
                                let ref target = self.listing[pos];
                                (target.0.clone(), target.2)
                            };

                            if is_dir {
                                self.stack.push(name);
                                self.load(fsts);
                            }  
                        }
                    }
                }

                Ok(rustbox::Event::KeyEvent(rustbox::keyboard::Key::Char('h'))) => {
                    if let Some(_) = self.stack.pop() {
                        self.load(fsts);
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
