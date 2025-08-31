use super::level::load_level_by_number;
use super::score::update_high_scores;
use super::menu::{draw_menu, draw_window};

const WIDTH: isize = 80;
const HEIGHT: isize = 25;

// const KEY_UP: u8 = 0x48;
// const KEY_DOWN: u8 = 0x50;
// const KEY_LEFT: u8 = 0x4B;
// const KEY_RIGHT: u8 = 0x4D;

static mut VGA_LOCAL: &mut isize = &mut 0;

// #[derive(Clone,Copy)]
// enum CellType {
//     Empty,
//     Wall,
//     Snake,
//     Food,
// }

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

#[derive(Clone,Copy)]
pub struct Point {
    pub x: usize,
    pub y: usize,
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

        unsafe {
            *VGA_LOCAL = 0;
        }

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

    fn draw(&self) {
        for i in 0..self.len {
            let ch = if i == 0 { b"@" } else { b"o" };
            //draw_char(self.x[i], self.y[i], ch, 0x0A);
            if let (Some(x), Some(y)) = (self.x.get(i), self.y.get(i)) {
                unsafe {
                    *VGA_LOCAL = 2 * (*y as isize * WIDTH + *x as isize);
                    crate::vga::write::string(VGA_LOCAL, ch, crate::vga::buffer::Color::Yellow);
                }
            }
        }
    }

    fn clear(&self) {
        for i in 0..self.len {
            //draw_char(self.x[i], self.y[i], b' ', 0x0F);
            if let (Some(x), Some(y)) = (self.x.get(i), self.y.get(i)) {
                unsafe {
                    *VGA_LOCAL = 2 * (*y as isize * WIDTH + *x as isize);
                    crate::vga::write::string(VGA_LOCAL, b" ", crate::vga::buffer::Color::Black);
                }
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

fn draw_food(x: usize, y: usize) {
    unsafe {
        *VGA_LOCAL = 2 * (y as isize * WIDTH + x as isize);
        crate::vga::write::string(VGA_LOCAL, b"*", crate::vga::buffer::Color::Green);
    }
}

fn draw_walls(walls: &[Point], ch: u8) {
    for wall in walls {
        if wall.x == 0 && wall.y == 0 {
            continue;
        }

        draw_char(wall.x, wall.y, ch, crate::vga::buffer::Color::Red);
    }
}

fn delay() {
    for _ in 0..1_000_000 {
        //core::hint::spin_loop();
        unsafe {
            core::arch::asm!("nop");
        }
    }
}

fn simple_hash(x: usize) -> usize {
    (x ^ 0x5f5f5f5f).wrapping_mul(2654435761)
}

fn draw_string(mut x: usize, y: usize, s: &[u8], color: crate::vga::buffer::Color) {
    for &c in s {
        draw_char(x, y, c, color);
        x += 1;
    }
}

fn write_number(mut x: usize, y: usize, mut value: usize) {
    // Handle 0 explicitly
    if value == 0 {
        draw_char(x, y, b'0', crate::vga::buffer::Color::White);
        return;
    }

    let mut digits = [0u8; 10]; // up to 10 digits
    let mut i = 0;

    while value > 0 {
        if let Some(d) = digits.get_mut(i) {
            *d = (value % 10) as u8 + b'0';
        }

        value /= 10;
        i += 1;
    }

    // Print in reverse order
    while i > 0 {
        i -= 1;

        if let Some(d) = digits.get(i) {
            draw_char(x, y, *d, crate::vga::buffer::Color::White);
        }

        x += 1;
    }
}

fn draw_char(x: usize, y: usize, ch: u8, color: crate::vga::buffer::Color) {
    unsafe {
        *VGA_LOCAL = 2 * (y as isize * WIDTH + x as isize);
        crate::vga::write::byte(VGA_LOCAL, ch, color);
    }
}

fn save_score_window() {
    clear_screen!();

    let menu = ["Please wait..."];
    unsafe {
        super::menu::SELECTED = 1;
    }

    draw_window(26, 6, 25, 8, Some("Save Score"));
    draw_menu(32, 9, &menu);
}


fn game_over() {
    clear_screen!();

    let menu = ["Game Over", "Back to menu"];
    unsafe {
        super::menu::SELECTED = 1;
    }

    draw_window(26, 6, 25, 10, Some("Game Over"));
    draw_menu(32, 9, &menu);

    loop {
        let scancode = crate::input::keyboard::keyboard_read_scancode();

        if scancode == 0x01 || scancode == 0x1C {
            break;
        }
    }
}

//
//
//

pub fn run() {
    let mut snake = Snake::new();

    let mut move_count = 0;
    let mut score = 0;

    let mut food_x = 20;
    let mut food_y = 10;

    //let mut field = [[CellType::Empty; WIDTH as usize]; HEIGHT as usize];

    let mut level = 1;

    let (mut walls, _) = load_level_by_number::<{ super::level::MAX_WALLS }>(level as u8);


    loop {
        let code = read_scancode();

        if code == 0x01 {
            clear_screen!();

            save_score_window();
            update_high_scores(score);
            game_over();
            break;
        }

        if code == 0xE0 {
            match read_scancode() {
                0x48 => snake.set_dir(Dir::Up),
                0x50 => snake.set_dir(Dir::Down),
                0x4B => snake.set_dir(Dir::Left),
                0x4D => snake.set_dir(Dir::Right),
                _ => {}
            }
        }

        if score % 30 == 0 && snake.len - 3 != 0 {
            level = (score / 30) + 1;

            draw_walls(&walls, b' ');
            (walls, _) = load_level_by_number::<{ super::level::MAX_WALLS }>(level as u8);

            snake.clear();
            snake = Snake::new();
        }

        move_count += 1;

        snake.clear();
        snake.step();

        // Check if snake ate the food
        if let (Some(x), Some(y)) = (snake.x.first(), snake.y.first()) {
            if *x == food_x && *y == food_y {
                if snake.len < MAX_LEN {
                    snake.len += 1;
                    score += 1;
                }

                let mut crt = 0;

                loop {
                    let seed = x * 13 + y * 31 + crt;
                    food_x = simple_hash(seed) % WIDTH as usize;
                    food_y = simple_hash(seed.wrapping_add(1)) % HEIGHT as usize;

                    if food_x == 0 || food_y == 0 {
                        crt += 1;
                        continue;
                    }

                    for wall in walls {
                        if food_x == wall.x && food_y == wall.y {
                            crt += 1;
                            continue;
                        }
                    }

                    break;
                }
            }

            for wall in walls {
                if wall.x == *x && wall.y == *y {
                    update_high_scores(score);
                    game_over();
                    return;
                }
            }
        }

        snake.draw();

        draw_walls(&walls, b'#');
        draw_food(food_x, food_y);

        draw_string(0, 0, b"Level: ", crate::vga::buffer::Color::White);
        write_number(7, 0, level as usize);
        draw_string(10, 0, b"Score: ", crate::vga::buffer::Color::White);
        write_number(17, 0, score as usize);
        draw_string(21, 0, b"Moves: ", crate::vga::buffer::Color::White);
        write_number(28, 0, move_count as usize);

        delay();
    }
}

