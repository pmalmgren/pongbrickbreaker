use ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE;
use ncurses::*;
use std::char;
use std::process;
use std::{thread, time};
use std::cmp;
use std::time::{SystemTime, UNIX_EPOCH};

// Should be divisible by 3 for [left, center, right]
const PADDLE_WIDTH: i32 = 12;

enum Direction {
    Left,
    Right,
    Down,
    Up,
    Still,
}

enum Command {
    Move(Direction),
    Quit,
}

impl Command {
    fn from_char(c: char) -> Command {
        match c {
            'a' => return Command::Move(Direction::Left),
            'd' => return Command::Move(Direction::Right),
            'q' => return Command::Quit,
            _ => return Command::Move(Direction::Still),
        };
    }

    fn from_i32(i: i32) -> Command {
        match char::from_u32(i as u32) {
            Some(ch) => return Command::from_char(ch),
            None => return Command::Move(Direction::Still), 
        };
    }
}

struct Point {
    x: i32,
    y: i32,
}

struct Bounds {
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
}

enum MoveResult {
    HitWallTopBottom,
    HitWallLeftRight,
    HitPaddleCenter,
    HitPaddleLeft,
    HitPaddleRight,
}

// Can be a ball, a paddle, or a brick.
struct GameObject {
    pos: Point,
    vel: Point,
    disp_char: u32,
    width: i32,
}

impl GameObject {
    fn get_bounds(&mut self) -> Bounds {
        let left_edge = self.pos.x - (self.width / 2);
        let right_edge = self.pos.x + (self.width / 2);
        Bounds { min_x: left_edge, max_x: right_edge, min_y: self.pos.y, max_y: self.pos.y }
    }
    // moves the game object by the direction and returns a collision
    fn push(&mut self, direction: Direction, bounds: &Bounds, object: Option<&Bounds>) -> Option<MoveResult> {
        let left_edge = self.pos.x - (self.width / 2);
        let right_edge = self.pos.x + (self.width / 2);
        return match direction {
            Direction::Left => {
                if left_edge > 1 {
                    self.pos.x -= 1;
                    return None;
                }
                Some(MoveResult::HitWallLeftRight)
            }
            Direction::Right => {
                if right_edge < (bounds.max_x - 2) {
                    self.pos.x += 1;
                    return None;
                }
                Some(MoveResult::HitWallLeftRight)
            },
            Direction::Up => {
                if self.pos.y > 1 {
                    self.pos.y -= 1;
                    return None;
                }
                Some(MoveResult::HitWallTopBottom)
            },
            Direction::Down => {
                if self.pos.y >= (bounds.max_y - 2) {
                    return Some(MoveResult::HitWallTopBottom)
                }

                return match object {
                    None => {
                        self.pos.y += 1;
                        None
                    },
                    Some(object) => {
                        if self.pos.y == object.max_y && self.pos.x < object.max_x && self.pos.x > object.min_x {
                            let third = PADDLE_WIDTH / 3;
                            if self.pos.x < (object.min_x + third) {
                                return Some(MoveResult::HitPaddleLeft);
                            }
                            if self.pos.x < (object.min_x + (2 * third)) {
                                return Some(MoveResult::HitPaddleCenter);
                            }
                            return Some(MoveResult::HitPaddleRight);
                        }
                        self.pos.y += 1;
                        None
                    }
                }
            },
            Direction::Still => None,
        }
    }

    // floats the game object by the velocity
    fn float(&mut self, bounds: &Bounds, paddle: &Bounds) {
        let x_collision: Option<MoveResult> = match self.vel.x {
            x if x < 0 => self.push(Direction::Left, bounds, Some(paddle)),
            x if x > 0 => self.push(Direction::Right, bounds, Some(paddle)),
            _ => None,
        };
        let y_collision: Option<MoveResult> = match self.vel.y {
            y if y > 0 => self.push(Direction::Down, bounds, Some(paddle)),
            y if y < 0 => self.push(Direction::Up, bounds, Some(paddle)),
            _ => None,
        };

        match y_collision {
            Some(MoveResult::HitPaddleCenter) => {
                self.vel.y = -self.vel.y;
                self.vel.x = 0;
            },
            Some(MoveResult::HitPaddleLeft) => {
                self.vel.x = -1;
                self.vel.y = -self.vel.y;
            },
            Some(MoveResult::HitPaddleRight) => {
                self.vel.x = 1;
                self.vel.y = -self.vel.y;
            },
            Some(MoveResult::HitWallTopBottom) => self.vel.y = -self.vel.y,
            Some(MoveResult::HitWallLeftRight) => self.vel.x = -self.vel.x,
            None => (),
        };

        match x_collision {
            Some(_collision) => self.vel.x = -self.vel.x,
            None => (),
        };

    }

    fn draw(&self) {
        let start = self.pos.x - self.width / 2;
        let end = self.pos.x + (self.width / 2);
        for x in start..cmp::max(end, start+1) {
            mvaddch(self.pos.y, x, self.disp_char);
        }
    }

    fn clear(&self) {
        let start = self.pos.x - self.width / 2;
        let end = self.pos.x + (self.width / 2);
        for x in start..cmp::max(end, start+1) {
            mvaddch(self.pos.y, x, ' ' as u32);
        }
    }
}

struct Game {
    bounds: Bounds,
    player: GameObject,
    ball: GameObject,
    window: WINDOW,
    last_ball_move: u128,
}

impl Game {
    fn draw_player(&mut self) {
        self.player.clear();
        self.player.draw();
    }

    fn draw_ball(&mut self) {
        self.ball.clear();
        self.ball.draw();
    }

    fn move_player(&mut self, direction: Direction) {
        self.player.clear();
        self.player.push(direction, &self.bounds, None);
        self.draw_player();
    }

    fn move_ball(&mut self) {
        let now = now_ms();
        if now - self.last_ball_move > 70 {
            self.last_ball_move = now;
            self.ball.clear();
            self.ball.float(&self.bounds, &self.player.get_bounds());
            self.draw_ball();
        }
    }
}

fn init() -> Result<WINDOW, String> {
    let window = initscr();
    cbreak();
    noecho();
    clear();
    refresh();

    keypad(window, true);
    nodelay(window, true);

    curs_set(CURSOR_INVISIBLE);

    if !has_colors() {
        endwin();
        return Err(String::from("No colors were available."));
    }

    start_color();

    attron(A_BOLD());
    box_(window, 0, 0);
    attroff(A_BOLD());
    
    return Ok(window);
}

fn run(game: &mut Game) {
    let ten_millis = time::Duration::from_millis(10);
 
    game.draw_player();
    game.draw_ball();
    refresh();

    loop {
        thread::sleep(ten_millis);
        match Command::from_i32(wgetch(game.window)) {
            Command::Move(direction) => {
                game.move_player(direction);
            },
            Command::Quit => break,
        };
        game.move_ball();
        refresh();
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
}

fn main() {
    let window = match init() {
        Ok(window) => window,
        Err(error) => {
            println!("Error creating window: {}\n", error);
            process::exit(1);
        },
    };

    let mut max_x: i32 = 0;
    let mut max_y: i32 = 0;
    getmaxyx(window, &mut max_y, &mut max_x);

    let mut game = Game {
        window: window,
        bounds: Bounds { max_x: max_x, max_y: max_y, min_x: 0, min_y: 0 },
        player: GameObject {
            pos: Point { x: (max_x / 2), y: max_y - 4},
            vel: Point { x: 0, y: 0 },
            disp_char: '=' as u32,
            width: PADDLE_WIDTH,
        },
        ball: GameObject {
            pos: Point { x: (max_x / 2), y: 2 },
            vel: Point { x: 0, y: 1 },
            disp_char: '*' as u32,
            width: 1,
        },
        last_ball_move: now_ms()
    };
    
    run(&mut game);

    endwin();
}
