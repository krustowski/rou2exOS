use crate::tui::{screen::Screen, app::TuiEvent};

pub trait Widget {
    fn render(&mut self, screen: &Screen, offset_x: usize, offset_y: usize);
    fn handle_event(&mut self, event: TuiEvent);
}

//
//  LABEL
//

pub struct Label {
    pub x: usize,
    pub y: usize,
    pub text: &'static str,
    pub attr: u8,
}

impl Widget for Label {
    fn render(&mut self, screen: &Screen, offset_x: usize, offset_y: usize) {
        let x = offset_x + self.x;
        let y = offset_y + self.y;
        for (i, byte) in self.text.bytes().enumerate() {
            Screen::write_char(offset_x + i, offset_y, byte, self.attr);
        }
    }

    fn handle_event(&mut self, _event: TuiEvent) {}
}

//
//  WINDOW
//

pub struct Window<'a> {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub title: Option<&'static str>,
    pub child: Option<&'a mut dyn Widget>,
}

impl<'a> Widget for Window<'a> {
    fn render(&mut self, screen: &Screen, offset_x: usize, offset_y: usize) {
        let attr = 0x4F;

        // Corners
        Screen::write_char(self.x, self.y, 0xC9, attr);
        Screen::write_char(self.x + self.w - 1, self.y, 0xBB, attr);
        Screen::write_char(self.x, self.y + self.h - 1, 0xC8, attr);
        Screen::write_char(self.x + self.w - 1, self.y + self.h - 1, 0xBC, attr);

        // Edges
        for i in 1..(self.w - 1) {
            Screen::write_char(self.x + i, self.y, 0xCD, attr);
            Screen::write_char(self.x + i, self.y + self.h - 1, 0xCD, attr);
        }

        for i in 1..(self.h - 1) {
            Screen::write_char(self.x, self.y + i, 0xBA, attr);
            Screen::write_char(self.x + self.w - 1, self.y + i, 0xBA, attr);
        }

        // Title
        if let Some(t) = self.title {
            let start = self.x + (self.w - t.len()) / 2;
            for (i, byte) in t.bytes().enumerate() {
                Screen::write_char(start + i, self.y, byte, 0x1E);
            }
        }

        //let base_x = offset_x + self.x;
        //let base_y = offset_y + self.y;

        // Render child widget
        if let Some(child) = &mut self.child {
            child.render(screen, offset_x, offset_y);
        }
    }

    fn handle_event(&mut self, event: TuiEvent) {
        if let Some(child) = &mut self.child {
            child.handle_event(event);
        }
    }
}

//
//  CONTAINER
//

pub struct Container<'a> {
    pub x: usize,
    pub y: usize,
    pub children: [&'a mut dyn Widget; 3], // up to 4 children
}

impl<'a> Widget for Container<'a> {
    fn render(&mut self, screen: &Screen, offset_x: usize, offset_y: usize) {
        let base_x = offset_x + self.x;
        let base_y = offset_y + self.y;

        for (i, child) in self.children.iter_mut().enumerate() {
            let dy = i * 2;
            let mut offset_child = OffsetWidget {
                widget: *child,
                dx: self.x,
                dy: self.y + dy,
            };
            offset_child.render(screen, base_x, base_y);
        }
    }

    fn handle_event(&mut self, event: TuiEvent) {
        for child in self.children.iter_mut() {
            // optionally dispatch only to focused child
            child.handle_event(event);
        }
    }
}

pub struct OffsetWidget<'a> {
    pub widget: &'a mut dyn Widget,
    pub dx: usize,
    pub dy: usize,
}

impl<'a> Widget for OffsetWidget<'a> {
    fn render(&mut self, screen: &Screen, offset_x: usize, offset_y: usize) {
        self.widget.render(screen, self.dx, self.dy); // ideally render with offset logic
        // To keep it simple now, we're skipping offset math
    }

    fn handle_event(&mut self, event: TuiEvent) {
        self.widget.handle_event(event);
    }
}

