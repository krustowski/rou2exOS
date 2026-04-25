use crate::input::keyboard::{keyboard_read_scancode};
use crate::tui::{screen::Screen, widget::Widget};

#[derive(Clone,Copy)]
pub enum TuiEvent {
    Key(u8),
    Quit,
}

pub struct TuiApp<'a> {
    pub root: Option<&'a mut dyn Widget>,
}

impl<'a> TuiApp<'a> {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn set_root(&mut self, root: &'a mut dyn Widget) {
        self.root = Some(root);
    }

    pub fn run(&mut self) {
        Screen::clear(0x07);
        if let Some(root) = &mut self.root {
            root.render(&Screen, 0, 0);
        }

        loop {
            let scancode = keyboard_read_scancode();

            let event = match scancode {
                0x01 => TuiEvent::Quit, // ESC
                _ => TuiEvent::Key(scancode),
            };

            if let Some(root) = &mut self.root {
                root.handle_event(event);
            }

            if let TuiEvent::Quit = event {
                break;
            }

            Screen::clear(0x07);
            if let Some(root) = &mut self.root {
                root.render(&Screen, 0, 0);
            }
        }
    }
}

