use std::io::{Write, stdout, stdin};
use std::time::{Instant, Duration};
use std::process::exit;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{RawTerminal,IntoRawMode};
use termion::color;
use termion::color::Color;
use std::thread;
use rand::Rng;
use rand::seq::SliceRandom;

const WIDTH: usize =10;
const HEIGHT: usize =22;
const HIDDEN: usize =2; //hide the first few lines
const OCC_CELL: &str ="░░";
const EMPTY_CELL: &str ="..";
const STARTING_LEVEL: u8 = 1;
const DOWN_TIMES: [u64; 21] = [ 1000, 793, 617, 472, 355, 262, 189, 135, 94, 64, 42, 28, 18, 11, 7, 4, 2, 1, 1, 1, 1]; 
//const COLORS: (Bg<Black>,Bg<LightBlack>,Bg<White>,Bg<Cyan>,Bg<Yellow>,Bg<Magenta>,Bg<Green>,Bg<Red>) =
//( color::Bg(color::Black), color::Bg(color::LightBlack),color::Bg(color::White),color::Bg(color::Cyan),color::Bg(color::Yellow),
//color::Bg(color::Magenta),color::Bg(color::Green),color::Bg(color::Green),color::Bg(color::Red),color::Bg(color::Blue));

fn main()

{

    let delta_time = Duration::from_millis(10); //time between game updates
    let lock_delay = Duration::from_millis(500); //line clear animation length
    let mut game = Game::new();

    print!("{}", termion::clear::All);
    game.drw_screen();
    game.drw_ui();

    
    

    let mut stdin = termion::async_stdin().keys();
    loop
    {
        let raw_input = stdin.next();
        if game.paused
        {
            if let Some(Ok(key)) = raw_input
            {
                match key 
                {
                    Key::Char('q') =>
                    {
                        print!("{}",termion::cursor::Show);
                        exit(0);
                    }
                    _=> { game.paused = false; }
                }
            }
        }
        else
        {
            if let Some(Ok(key)) = raw_input
            {
                match key 
                {
                    Key::Char('q') =>
                    {
                        print!("{}",termion::cursor::Show);
                        exit(0);
                    }
                    /*Key::Esc => { game.pause(); }*/
                    Key::Right => { game.mv_current((1,0)); }
                    Key::Left => { game.mv_current((-1,0)); }
                    Key::Down => { game.mv_current((0,1)); }
                    Key::Up => { game.rot_current(true); }
                    Key::Char('h') => { game.hold();}
                    Key::Char('r') => { game.rot_current(false); }

                    Key::Char(';') => { game.mv_current((1,0)); }
                    Key::Char('j') => { game.mv_current((-1,0)); }
                    Key::Char('k') => { game.mv_current((0,1)); }
                    Key::Char('l') => { game.rot_current(true); }

                    Key::Esc => { game.pause(); }
                    _=> {}
                }
            }
            //if tetro has hit ground and collision time has elapsed, lock it, check for line
            //clears, add xp, check for level change, then spawn the next tetro
            if game.locking && game.when_grounded.elapsed() >= lock_delay
            {
                game.next();
            }

            //move tetro down every down_time
            if game.last_down.elapsed() >= game.down_time
            {
                game.update();
            }
        }
        thread::sleep(delta_time); //TODO: account for the time taken to compute the game
    }
}
struct Game
{
    last_down: Instant,
    when_grounded: Instant,
    locking: bool,
    level: u8,
    down_time: Duration,
    score: u32,
    xp: u16,
    tetros: [Tetro;7],
    bag: Vec<Tetro>,
    colors: Box<[Box<dyn Color>]>,
    tetro: Tetro,
    held_tetro: Option<Tetro>,
    stdout: RawTerminal<std::io::Stdout>,
    board: Board,
    line_clear_time: Duration,
    holding: bool,
    paused: bool,
}
impl Game
{
    fn new() -> Game
    {
        let line_clear_time = Duration::from_millis(300); //line clear animation length
        let mut paused = false;

        let mut last_down = Instant::now();
        let mut when_grounded = Instant::now();
        let mut locking= false;
        let mut level = STARTING_LEVEL;
        let mut down_time = Duration::from_millis(DOWN_TIMES[(level-1) as usize]); //time between each down movement of the piece
        let mut score: u32 = 0;
        let mut xp: u16 = 0;
        let mut holding = false;
        
        let sp = (5,HIDDEN as i16);
        let mut tetros = [Tetro::I(sp),Tetro::J(sp),Tetro::L(sp),Tetro::O(sp),Tetro::S(sp),Tetro::Z(sp),Tetro::T(sp)];
        let mut bag = Vec::new();
        restock(&mut bag, &mut tetros);
        //let mut tetro = bag[bag.len()-1];
        //bag.pop();
        let mut tetro = Tetro::Z(sp);
        let mut stdout = stdout().into_raw_mode().unwrap();
        let mut board = Board::new();
        let mut held_tetro: Option<Tetro> = None;
        
        let colors: [Box<dyn Color>;8] =
        [
            Box::new(color::Black),Box::new(color::White),Box::new(color::Red),Box::new(color::Blue),
            Box::new(color::Green),Box::new(color::Magenta), Box::new(color::Cyan),
            Box::new(color::Yellow)
        ];

        Game
        {
            last_down: last_down, when_grounded: when_grounded, locking: locking, level: level,
            down_time: down_time, score: score, xp: xp, tetros: tetros, bag: bag, tetro: tetro,
            stdout: stdout, board: board, line_clear_time: line_clear_time, colors: Box::new(colors),
            held_tetro: held_tetro, holding: holding, paused: paused,
        }
    }
    fn new_tetro(&mut self) -> Tetro
    {
        let tetro = self.bag[self.bag.len()-1];
        self.bag.pop();
        if self.bag.len() == 0 { restock(&mut self.bag, &mut self.tetros) }
        return tetro;
    }
    fn spawn_tetro(&mut self, tetro: &Tetro) //might fuck up, TODO: fix this function
    {
        let mut new_tetro = *tetro;
        new_tetro.pos = (5, HIDDEN as i16);
        if self.board.validate(&new_tetro) == Sts::Collision //if game.tetro placement is invalid, try a few alternate placements
        {
            for mv in [new_tetro.mv((0,-1)),new_tetro.mv((1,0)),new_tetro.mv((-1,0)),new_tetro.mv((0,-2)),new_tetro.mv((-1,-1)),new_tetro.mv((1,-1)),new_tetro.mv((0,-3))]
            {
                if self.board.validate(&mv) != Sts::Collision
                { self.tetro = mv;
                    break;
                }
            }
            self.game_over();
        }
        else
        { self.tetro = new_tetro; }
        self.last_down = Instant::now();
    }
    fn next(&mut self)
    {
        self.locking = false;
        self.holding = false;
        self.board.lock_tetro(&self.tetro, true);

        let tris = self.board.check_lines(self.tetro.pos.1 as usize);
        if tris.len() > 0 //do this stuff if there's a line clear:
        {
            for line in &tris //clear lines and apply gravity to the lines above
            {
                self.drw_line(*line as u8, false);
                thread::sleep(self.line_clear_time);
                self.board.gravitate((0,*line),1);
            }
           
            let l32 = self.level as u32;
            let inc = match tris.len() //get xp and score change
            {
                1 => { (1,40*l32) } 2 => { (3,100*l32) }
                3 => { (5,300*l32) } 4 => { (8,1200*l32) }
                _ =>{ (0,0) }
            };
            self.xp += inc.0;
            self.score += inc.1;
            let goal: u16 = 5*(self.level as u16);

            if self.xp >= goal //level change
            { 
                self.level += 1;
                self.xp -= goal;
                self.down_time = Duration::from_millis(DOWN_TIMES[(self.level-1) as usize]); //speed up
            }
        }
        let new_tetro = self.new_tetro();
        self.spawn_tetro(&new_tetro);
       
        self.drw_screen();
        self.drw_ui();
    }
    fn mv_current (&mut self, vec: (i16,i16))
    {
        let mv = self.tetro.mv(vec);
        let sts = self.board.validate(&mv);
        if sts == Sts::Good
        {
            self.when_grounded = Instant::now();
            self.tetro = mv;
            self.drw_screen();
        } 
    }
    fn rot_current (&mut self, cw: bool)
    {
        let mut rot = self.tetro.rot(true);
        for mv in [rot,rot.mv((0,1)),rot.mv((0,-1)),rot.mv((1,0)),rot.mv((-1,0))] {
            if self.board.validate(&mv) == Sts::Good
            {
                self.tetro = mv;
                self.drw_screen();
                self.when_grounded = Instant::now();
                break;
            }
        }
    }
    fn hold (&mut self)
    {
        if self.holding == false
        {
            match self.held_tetro
            {
                None => { self.held_tetro = Some(self.new_tetro()); }
                _=> {}
            }
            let tmp = self.tetro;
            self.spawn_tetro(&self.held_tetro.unwrap());
            self.held_tetro = Some(tmp);
            self.holding = true;
            self.drw_screen();
            self.drw_ui();
        }
    }
    fn update (&mut self)
    {
        self.mv_current((0,1));
        let mv = self.tetro.mv((0,1));
        let sts = self.board.validate(&mv);
        if sts == Sts::Collision
        {
            self.locking = true;
            if self.tetro.pos.1 <= 1
            { self.game_over(); }
        } 
        self.last_down = Instant::now();
    }
    fn drw_screen(&mut self)
    {
        print!("{}{}",
               termion::cursor::Goto(1,1),
               termion::cursor::Hide,);
        for y in HIDDEN..HEIGHT 
        {
            for x in 0..WIDTH
            {
                if self.board.tiles[y][x]
                { self.drw_clr((x as u8,y as u8),self.board.colors[y][x]); }
                else
                { self.drw_clr((x as u8,y as u8),0); }
            }
        }
        self.drw_tetro(self.tetro, true);
    }
    fn drw_ui(&mut self)
    {
        self.drw_text(1,"level: ",self.level as i32);
        self.drw_text(2,"score: ",self.score as i32);
        //self.drw_text(3,"xp: ",self.xp as i32);
        
        let bag = self.bag.clone();
        let next = bag[bag.len()-1];
        self.drw_next_held(3, "next: ", next);
        if self.held_tetro.is_none() == false
        { self.drw_next_held(16, "hold: ", self.held_tetro.unwrap()); }
    }
    fn drw_text(&mut self, line: u16, label: &str, value: i32)
    {
        let x = 2*(WIDTH as u16)+3;
        print!("{}{}{}",termion::cursor::Goto(x,line),label,value);
        self.stdout.flush().unwrap();
    }
    fn drw_next_held(&mut self, line: u8, label: &str, tetro: Tetro)
    {
        let y = line+2;
        let x: u8 = WIDTH as u8 + 1;
        let mut drw_tetro = tetro;
        drw_tetro.pos = (x as i16,(y) as i16);
        
        print!("{}{}",termion::cursor::Goto((2*x+1) as u16,line as u16),label);
        
        for ty in 0..4
        {   for tx in 0..4
            { self.drw_clr((tx as u8 + x,ty as u8 + y), 0) }
        }
        self.drw_tetro(drw_tetro,true);
    }
    fn drw_tetro(&mut self, tetro: Tetro, occ: bool)
    {
        let tiles = tetro.real_tiles();
        for tile in tiles
        {
            if occ
            { self.drw_clr((tile.0 as u8, tile.1 as u8), tetro.color) }
            else
            { self.drw_clr((tile.0 as u8, tile.1 as u8), 1 ) }
        }
    }
    fn drw_line(&mut self, line: u8, occ: bool)
    {
        if occ
        { for i in 0..WIDTH as u8 { self.drw_clr((i,line),1); } }
        else
        { for i in 0..WIDTH as u8 { self.drw_clr((i,line),0); } }
    }
    fn drw_tile(&mut self, pos:(u8,u8), occ: bool)
    {
        if pos.1 >= HIDDEN as u8 //dont do anything if the tile is offscreen
        {
            //let col = if occ { colors[0] } else { colors[1] };
            let pix: &str = if occ { OCC_CELL } else { EMPTY_CELL };
            print!("{}{}",
                   termion::cursor::Goto((2*pos.0+1).into(),(pos.1+1-HIDDEN as u8).into()),
                   pix );
            self.stdout.flush().unwrap();
        }
    }
    fn drw_clr(&mut self, pos:(u8,u8), color: u8)
    {
        if pos.1 >= HIDDEN as u8 //dont do anything if the tile is offscreen
        {
            let pix: &str = if color <= 10 { "  " } else { "░░" };
            let c = if color <= 10 { color } else { color-10 };
                
            //let col = if occ { colors[0] } else { colors[1] };
            print!("{}{}{}{}",
                   termion::cursor::Goto((2*pos.0+1).into(),(pos.1+1-HIDDEN as u8).into()),
                   color::Bg(&*self.colors[c as usize]),
                   pix,
                   color::Bg(color::Reset)
                   );
            self.stdout.flush().unwrap();
        }
    }
    fn game_over(&mut self)
    {
        /*let y=8;
        for x in 0..WIDTH
        {
            print!("{}  ", termion::cursor::Goto((2*x+1) as u16,y));
        }
        print!("{}game over", termion::cursor::Goto((WIDTH-5) as u16,y));
        self.stdout.flush().unwrap();
        let stdin = stdin();
        for key in stdin.keys() {
            match key.unwrap() {
                _=> 
                {*/
                    print!("{}",termion::cursor::Show);
                    exit(0);
                /*}
            }
        }*/
    }
    fn pause(&mut self)
    {
        let y=8;
        for x in 0..WIDTH
        {
            print!("{}  ", termion::cursor::Goto((2*x+1) as u16,y));
        }
        print!("{}paused", termion::cursor::Goto((WIDTH-3) as u16,y));
        self.stdout.flush().unwrap();
        self.paused = true;
    }
} 


fn restock(bag: &mut Vec<Tetro>, tetros: &mut[Tetro]) //copy tetros to bag with random order
{
    let mut rng = rand::thread_rng();
    tetros.shuffle(&mut rng);
    *bag = Vec::new(); //just in case
    for tetro in tetros { bag.push(*tetro); }

}

//tetrimino is too long of a word to be writing everywhere
#[derive(Copy,Clone)]
struct Tetro
{ tiles: [(u8,u8);4], width: u8, height: u8, pos: (i16,i16), color: u8 }
impl Tetro
{
    fn new(tiles: [(u8,u8);4], pos: (i16,i16), color: u8) -> Tetro
    {
        let mut width: u8 = 0;
        let mut height: u8 = 0;
        for tile in tiles {
            if tile.0 > width {
                width = tile.0
            }
            if tile.1 > height {
                height = tile.1
            }
        }
        Tetro
        {
            tiles: tiles,
            width: width,
            height: height,
            pos: pos,
            color: color,
        }
    }
    fn I(pos: (i16, i16)) -> Tetro {Tetro::new([(0,0),(0,1),(0,2),(0,3)],pos, 6)}
    fn J(pos: (i16, i16)) -> Tetro {Tetro::new([(1,0),(1,1),(1,2),(0,2)],pos, 3)}
    fn L(pos: (i16, i16)) -> Tetro {Tetro::new([(0,0),(0,1),(0,2),(1,2)],pos, 7)}
    fn O(pos: (i16, i16)) -> Tetro {Tetro::new([(0,0),(1,0),(1,1),(0,1)],pos, 1)}
    fn S(pos: (i16, i16)) -> Tetro {Tetro::new([(0,1),(1,1),(1,0),(2,0)],pos, 4)}
    fn Z(pos: (i16, i16)) -> Tetro {Tetro::new([(0,0),(1,0),(1,1),(2,1)],pos, 2)}
    fn T(pos: (i16, i16)) -> Tetro {Tetro::new([(0,1),(1,0),(1,1),(2,1)],pos, 5)}

    //mv and rot don't move or rotate the tetro in place, instead return a moved or rotated copy of
    //the tetro. it's more useful that way
    fn mv(&mut self, vec: (i16,i16)) -> Tetro
    {
        let newpos=(self.pos.0+vec.0,self.pos.1+vec.1);
        Tetro::new(self.tiles,newpos,self.color)
    }
    fn rot(&mut self, cw: bool) -> Tetro
    {
        let mut new_tiles: [(u8,u8);4] = [(0,0);4];
        for i in 0..4
        {
                if cw {
                    new_tiles[i] = (/*self.tetro.height-1-*/self.tiles[i].1, self.width-self.tiles[i].0);
                } else {
                    new_tiles[i] = (self.height-self.tiles[i].1, /*self.tetro.width-1*/self.tiles[i].0);
                }
        }
        Tetro::new(new_tiles,self.pos,self.color)
    }
    fn real_tiles(&self) -> [(i16,i16);4]
    {
        let mut real_tiles: [(i16,i16);4] = [(0,0);4];
        for i in 0..4
        {
            real_tiles[i] = ((self.tiles[i].0 as i16)+self.pos.0, (self.tiles[i].1 as i16)+self.pos.1);
        }
        return real_tiles;
     }
}

struct Board
{ tiles: [[bool;WIDTH];HEIGHT], colors: [[u8;WIDTH];HEIGHT] }
impl Board
{
    fn new() -> Board
    {
        Board { tiles: [[false;WIDTH];HEIGHT], colors: [[0;WIDTH];HEIGHT] }
    }
    //check if a tetro placement is valid
    fn validate(&mut self, tetro: &Tetro) -> Sts 
    {
        let w = WIDTH as i16;
        let h = HEIGHT as i16;
        for tile in tetro.real_tiles()
        {
            if tile.0 < 0 || tile.0 >= w
            { return Sts::Invalid; }
            else if tile.1 >= h
            { return Sts::Collision; }
            else if tile.1 >= 0 && self.tiles[tile.1 as usize][tile.0 as usize]
            { return Sts::Collision; }
        }
        return Sts::Good;
    }

    fn lock_tetro(&mut self, tetro: &Tetro, occ: bool)
    {
        for tile in tetro.real_tiles()
        {
            self.tiles[tile.1 as usize][tile.0 as usize] = occ;
            self.colors[tile.1 as usize][tile.0 as usize] = tetro.color;
        }
    }
    //check for a line clear
    fn check_lines(&mut self, max: usize) -> Vec<usize>
    {
        let mut cleared_lines = Vec::<usize>::new();
        for i in max..HEIGHT
        {
            let mut cleared = true;
            for tile in self.tiles[i]
            { 
                if tile == false { cleared = false; }
            }
            if cleared
            {
                cleared_lines.push(i);
            }
        }
        return cleared_lines;
    }
    //move lines down after a line clear
    fn gravitate(&mut self, lines: (usize,usize), distance: usize)
    {
        for i in (lines.0+1..lines.1+distance).rev()
        {
            self.tiles[i] = self.tiles[i-distance];
            self.colors[i] = self.colors[i-distance];
        }
    }
}

    

#[derive(PartialEq)]
enum Sts {
    Good,
    Invalid,
    Collision,
}
