const WIDTH: isize = 80;
const HEIGHT: isize = 25;

const KEY_UP: u8 = 0x48;
const KEY_DOWN: u8 = 0x50;
const KEY_LEFT: u8 = 0x4B;
const KEY_RIGHT: u8 = 0x4D;

fn read_scancode() -> u8 {
    // Wait until the keyboard has data (bit 0 set in status port 0x64)
    while inb(0x64) & 1 == 0 {}
    inb(0x60)
}

fn inb(port: u16) -> u8 {
    let ret: u8;
    unsafe {
        core::arch::asm!("in al, dx", in("dx") port, out("al") ret);
    }
    ret
}

const MAX_LEN: usize = 64;

#[derive(Copy, Clone, PartialEq)]
enum Dir { Up, Down, Left, Right }

struct Snake {
    x: [usize; MAX_LEN],
    y: [usize; MAX_LEN],
    len: usize,
    dir: Dir,
}

impl Snake {
    fn new() -> Self {
        let mut x = [0; MAX_LEN];
        let mut y = [0; MAX_LEN];

        if let Some(xx) = x.get_mut(0) {
            *xx = 40;
        };
        if let Some(yy) = y.get_mut(0) {
            *yy = 12;
        };

        Self { x, y, len: 3, dir: Dir::Right }
    }

    fn step(&mut self) {
        // Shift body
        for i in (1..self.len).rev() {
            if let (Some(&prev_x), Some(&prev_y)) = (self.x.get(i.wrapping_sub(1)), self.y.get(i.wrapping_sub(1))) {
                if let (Some(x), Some(y)) = (self.x.get_mut(i), self.y.get_mut(i)) {
                    *x = prev_x;
                    *y = prev_y;
                }
            }

            //self.x[i] = self.x[i - 1];
            //self.y[i] = self.y[i - 1];
        }

        // Move head
        match self.dir {
            Dir::Up    => 
                if let Some(y) = self.y.get_mut(0) {
                    if *y > 0 {
                        *y -= 1;
                    }
                }
            Dir::Down  => 
                if let Some(y) = self.y.get_mut(0) {
                    if *y < HEIGHT as usize - 1 {
                        *y += 1;
                    }
                }
            Dir::Left  => 
                if let Some(x) = self.x.get_mut(0) {
                    if *x > 0 {
                        *x -= 1;
                    }
                }
            Dir::Right => 
                if let Some(x) = self.x.get_mut(0) {
                    if *x < WIDTH as usize - 1 {
                        *x += 1;
                    }
                }
        }
    }

    fn draw(&self, vga_index: &mut isize) {
        for i in 0..self.len {
            let ch = if i == 0 { b"@" } else { b"o" };
            //draw_char(self.x[i], self.y[i], ch, 0x0A);
            if let (Some(x), Some(y)) = (self.x.get(i), self.y.get(i)) {
                *vga_index = 2 * (*y as isize * WIDTH + *x as isize);
                crate::vga::write::string(vga_index, ch, crate::vga::buffer::Color::Yellow);
            }
        }
    }

    fn clear(&self, vga_index: &mut isize) {
        for i in 0..self.len {
            //draw_char(self.x[i], self.y[i], b' ', 0x0F);
            if let (Some(x), Some(y)) = (self.x.get(i), self.y.get(i)) {
                *vga_index = 2 * (*y as isize * WIDTH + *x as isize);
                crate::vga::write::string(vga_index, b" ", crate::vga::buffer::Color::Black);
            }
        }
    }

    fn set_dir(&mut self, new: Dir) {
        // Prevent reversing direction
        if (self.dir == Dir::Up && new != Dir::Down)
            || (self.dir == Dir::Down && new != Dir::Up)
                || (self.dir == Dir::Left && new != Dir::Right)
                || (self.dir == Dir::Right && new != Dir::Left) {
                    self.dir = new;
        }
    }
}

fn draw_food(x: usize, y: usize, vga_index: &mut isize) {
    *vga_index = 2 * (y as isize * WIDTH + x as isize);
    crate::vga::write::string(vga_index, b"*", crate::vga::buffer::Color::Pink);
}

fn delay() {
    for _ in 0..1_000_000 {
        //core::hint::spin_loop();
        unsafe {
            core::arch::asm!("nop");
        }
    }
}

//
//
//

pub fn run(vga_index: &mut isize) {
    let mut snake = Snake::new();

    let mut food_x = 20;
    let mut food_y = 10;

    loop {
        if read_scancode() == 0xE0 {
            match read_scancode() {
                0x48 => snake.set_dir(Dir::Up),
                0x50 => snake.set_dir(Dir::Down),
                0x4B => snake.set_dir(Dir::Left),
                0x4D => snake.set_dir(Dir::Right),
                _ => {}
            }
        }

        snake.clear(vga_index);
        snake.step();

        // Check if snake ate the food
        if let (Some(x), Some(y)) = (snake.x.get(0), snake.y.get(0)) {
            if *x == food_x && *y == food_y {
                if snake.len < MAX_LEN {
                    snake.len += 1;
                }

                // Simple food reposition (not random yet)
                food_x = (food_x + 8) % WIDTH as usize;
                food_y = (food_y + 4) % HEIGHT as usize;
            }
        }

        snake.draw(vga_index);
        draw_food(food_x, food_y, vga_index);

        delay();
    }
}

pub fn run_old(vga_index: &mut isize) {
    let mut x = 40;
    let mut y = 12;

    loop {
        *vga_index = 2 * (y * WIDTH + x);
        crate::vga::write::string(vga_index, b"@", crate::vga::buffer::Color::Yellow);
        //draw(x, y, b'@', 0x1f); // draw white '@' on blue

        let code = read_scancode();

        if code == 27 {
            break;
        }

        if code == 0xE0 {
            let next = read_scancode();

            *vga_index -= 2;
            crate::vga::write::string(vga_index, b"@", crate::vga::buffer::Color::Black);
            //draw(x, y, b' ', 0x0f); // erase old position

            match next {
                KEY_UP => if y > 0         { y -= 1; },       // Up
                KEY_DOWN => if y < HEIGHT-1 { y += 1; },       // Down
                KEY_LEFT => if x > 0         { x -= 1; },       // Left
                KEY_RIGHT => if x < WIDTH-1  { x += 1; },       // Right
                _ => {}
            }
        }
    }
}
