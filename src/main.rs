use ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE;
use ncurses::*;
use std::char;
use std::process;
use std::{thread, time};
use std::cmp;
use std::time::{SystemTime, UNIX_EPOCH};
use std::convert::TryFrom;

// Should be divisible by 3 for [left, center, right]
const PADDLE_WIDTH: i32 = 12;
const NUM_ROWS: i32 = 4;
const BRICKS_PER_ROW: i32 = 6;

enum Direction {
    Left,
    Right,
    Down,
    Up,
    Still,
}

impl Direction {
    fn vel(&self) -> Point {
        match self {
            Direction::Left => Point { x: -1, y: 0 },
            Direction::Right => Point { x: 1, y: 0 },
            Direction::Up => Point { x: 0, y: -1 },
            Direction::Down => Point { x: 0, y: 1 },
            Direction::Still => Point { x: 0, y: 0 },
        }
    }
}

struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn will_collide(&self, bounds: &Bounds, direction: &Direction) -> bool {
        let vel = direction.vel();
        // every object in our game has a height of 1
        (self.y + vel.y) == bounds.max_y && (self.x + vel.x) <= bounds.max_x && (self.x + vel.x) >= bounds.min_x
    }

    fn will_collide_with_any(&self, bounds: &Vec<Bounds>, direction: &Direction) -> Option<usize> {
        for (idx, bound) in bounds.iter().enumerate() {
            if self.will_collide(bound, direction) {
                return Some(idx);
            }
        }
        None
    }

    fn move_dir(&mut self, direction: &Direction) {
        let vel = direction.vel();
        self.x += vel.x;
        self.y += vel.y;
    }
}

#[allow(dead_code)]
struct Bounds {
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
}

enum MoveResult {
    HitPaddleCenter,
    HitPaddleLeft,
    HitPaddleRight,
    HitWallLeftRight,
    HitWallBottom,
    HitWallTop,
    HitBrick(Direction, usize),
}

// Can be a ball, a paddle, or a brick.
struct GameObject {
    pos: Point,
    vel: Point,
    disp_char: u32,
    width: i32,
}

impl GameObject {
    fn get_bounds(&self) -> Bounds {
        let left_edge = self.pos.x - (self.width / 2);
        let right_edge = self.pos.x + (self.width / 2);
        Bounds { min_x: left_edge, max_x: right_edge, min_y: self.pos.y, max_y: self.pos.y }
    }

    fn do_move1(&mut self, bricks: &Vec<Bounds>, dir: Direction) -> Option<MoveResult> {
        match self.pos.will_collide_with_any(bricks, &dir) {
            Some(idx) => Some(MoveResult::HitBrick(dir, idx)),
            None => {
                self.pos.move_dir(&dir);
                None
            }
        }
    }

    // moves the game object by the direction and returns a collision with the paddle or bricks
    fn move1(&mut self, direction: Direction, bounds: &Bounds, paddle_bounds: &Bounds, bricks: &Vec<Bounds>) -> Option<MoveResult> {
        let left_edge = self.pos.x - (self.width / 2);
        let right_edge = self.pos.x + (self.width / 2);

        return match direction {
            Direction::Left => {
                if left_edge <= 1 {
                    return Some(MoveResult::HitWallLeftRight);
                }

                self.do_move1(bricks, direction)
            },
            Direction::Right => {
                if right_edge >= (bounds.max_x - 2) {
                    return Some(MoveResult::HitWallLeftRight)
                }

                self.do_move1(bricks, direction)
            },
            Direction::Up => {
                if self.pos.y <= 1 {
                    return Some(MoveResult::HitWallTop);
                }

                self.do_move1(bricks, direction)
            },
            Direction::Down => {
                if self.pos.y >= (bounds.max_y - 2) {
                    return Some(MoveResult::HitWallBottom)
                }

                if self.pos.will_collide(paddle_bounds, &Direction::Down) {
                    let third = PADDLE_WIDTH / 3;
                    if self.pos.x < (paddle_bounds.min_x + third) {
                        return Some(MoveResult::HitPaddleLeft);
                    }
                    if self.pos.x < (paddle_bounds.min_x + (2 * third)) {
                        return Some(MoveResult::HitPaddleCenter);
                    }
                    return Some(MoveResult::HitPaddleRight);
                }

                self.do_move1(bricks, direction)
            },
            Direction::Still => None,
        }
    }

    // floats the game object by the velocity
    fn float(&mut self, screen_bounds: &Bounds, paddle_bounds: &Bounds, brick_bounds: &Vec<Bounds>) -> Result<Option<usize>, String> {
        let mut hit_brick: Option<usize> = None;
        let mut lost: bool = false;
        let x_collision: Option<MoveResult> = match self.vel.x {
            x if x < 0 => self.move1(Direction::Left, screen_bounds, paddle_bounds, brick_bounds),
            x if x > 0 => self.move1(Direction::Right, screen_bounds, paddle_bounds, brick_bounds),
            _ => None,
        };
        let y_collision: Option<MoveResult> = match self.vel.y {
            y if y > 0 => self.move1(Direction::Down, screen_bounds, paddle_bounds, brick_bounds),
            y if y < 0 => self.move1(Direction::Up, screen_bounds, paddle_bounds, brick_bounds),
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
            Some(MoveResult::HitWallTop) => self.vel.y = -self.vel.y,
            Some(MoveResult::HitWallBottom) => {
                self.vel.x = 0;
                self.vel.y = 0;
                lost = true;
            },
            Some(MoveResult::HitBrick(Direction::Down, brick_idx)) => {
                self.vel.y = -self.vel.y;
                hit_brick = Some(brick_idx);
            },
            Some(MoveResult::HitBrick(Direction::Up, brick_idx)) => {
                self.vel.y = -self.vel.y;
                hit_brick = Some(brick_idx);
            },
            _ => (),
        };

        match x_collision {
            Some(MoveResult::HitBrick(Direction::Left, brick_idx)) => {
                if !hit_brick.is_some() {
                    self.vel.x = -self.vel.x;
                    hit_brick = Some(brick_idx);
                }
            },
            Some(MoveResult::HitBrick(Direction::Right, brick_idx)) => {
                if !hit_brick.is_some() {
                    self.vel.x = -self.vel.x;
                    hit_brick = Some(brick_idx);
                }
            },
            Some(MoveResult::HitWallLeftRight) => self.vel.x = -self.vel.x,
            Some(_collision) => self.vel.x = -self.vel.x,
            None => (),
        };

        if lost {
            return Err("Player has lost.".to_string());
        }
        Ok(hit_brick)
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
    bricks: Vec<GameObject>,
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

    fn draw_bricks(&mut self) {
        for brick in &self.bricks {
            brick.draw()
        }
    }

    fn move_player(&mut self, direction: Direction) {
        self.player.clear();
        self.player.move1(direction, &self.bounds, &self.bounds, &vec![]);
        self.draw_player();
    }

    fn move_ball(&mut self) -> Result<Option<usize>, String> {
        let now = now_ms();
        if now - self.last_ball_move > 70 {
            let brick_bounds = self.get_brick_bounds();
            self.last_ball_move = now;
            self.ball.clear();
            let result = self.ball.float(&self.bounds, &self.player.get_bounds(), &brick_bounds);
            self.draw_ball();
            return result;
        }
        Ok(None)
    }

    fn get_brick_bounds(&self) -> Vec<Bounds> {
        let mut brick_bounds = Vec::with_capacity(self.bricks.len());
        for brick in &self.bricks {
            let bounds = brick.get_bounds();
            brick_bounds.push(bounds);         
        }
        brick_bounds
    }

    fn rm_brick(&mut self, brick_idx: usize) {
        assert!(brick_idx <= self.bricks.len());
        self.bricks[brick_idx].clear();
        self.bricks.remove(brick_idx);
    }
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

    init_pair(1, COLOR_GREEN, COLOR_BLACK);
    wbkgdset(window, COLOR_PAIR(1));

    attron(A_BOLD());
    box_(window, 0, 0);
    attroff(A_BOLD());
    
    return Ok(window);
}

fn run(game: &mut Game) -> String {
    let ten_millis = time::Duration::from_millis(10);
 
    game.draw_bricks();
    game.draw_player();
    game.draw_ball();
    refresh();

    loop {
        thread::sleep(ten_millis);
        match Command::from_i32(wgetch(game.window)) {
            Command::Move(direction) => {
                game.move_player(direction);
            },
            Command::Quit => return "Bye!".to_string(),
        };
        let result = game.move_ball();
        match result {
            Ok(hit_brick) => {
                match hit_brick {
                    Some(brick_idx) => game.rm_brick(brick_idx),
                    None => (),
                }
            },
            Err(_) => {
                return "You lost :(".to_string();
            }
        }

        if game.bricks.len() == 0 {
            return "You won! :)".to_string();
        }
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

    let brick_width = max_x / BRICKS_PER_ROW;
    let capacity = usize::try_from((BRICKS_PER_ROW - 1) * NUM_ROWS).unwrap();
    let mut bricks = Vec::with_capacity(capacity);
    for row in 1..NUM_ROWS+1 {
        let offset = match row % 2 {
            0 => (brick_width / 2),
            _ => (brick_width / 2) - (brick_width / 4),
        };
        for col in 0..(BRICKS_PER_ROW - 1) {
            bricks.push(
                GameObject {
                    pos: Point { x: offset + (col * brick_width) + (brick_width / 2), y: row },
                    vel: Point { x: 0, y: 0 },
                    disp_char: '#' as u32,
                    width: brick_width,
                }
            );
        }
    }

    let mut game = Game {
        window: window,
        bounds: Bounds { max_x: max_x, max_y: max_y, min_x: 0, min_y: 0 },
        // we want the paddle to be above the bottom border of the screen
        player: GameObject {
            pos: Point { x: (max_x / 2), y: max_y - 4},
            vel: Point { x: 0, y: 0 },
            disp_char: '=' as u32,
            width: PADDLE_WIDTH,
        },
        ball: GameObject {
            pos: Point { x: (max_x / 2), y: 7 },
            vel: Point { x: 0, y: 1 },
            disp_char: '0' as u32,
            width: 1,
        },
        bricks: bricks,
        last_ball_move: now_ms()
    };
    
    let msg = run(&mut game);

    endwin();
    println!("{}", msg);
}
