use crossterm::{
    ExecutableCommand, QueueableCommand, cursor,
    event::{Event, KeyCode, KeyEventKind, poll, read},
    execute,
    terminal::{self, Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use rand::Rng;
use std::{
    io::{self, Stdout, Write, stdout},
    thread,
    time::Duration,
};

const FPS: f64 = 30.0;
const FALLING_SPD: i32 = FPS as i32; // fallspeed counted with: FPS / FALLING_SPD
const LEXICON: &str = include_str!("lexicon.txt");
const MAX_WORDS_IN_FRAME: usize = 3;
const HEALTH_CHAR: &str = "o";

#[derive(Clone)]
struct Word {
    text: String,
    pos: (u16, u16), // col, row
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let stdout = stdout();
    let u_words: Vec<String> = LEXICON.lines().map(|s| s.to_string()).collect();
    let mut game: Game = Game::new(stdout, u_words);

    game.setup()?;
    game.intro()?;

    while !game.quit {
        game.game_loop()?;
        thread::sleep(game.fps);
    }

    disable_raw_mode()?;
    Ok(())
}

struct Game {
    so: Stdout,
    input: String,
    u_words: Vec<String>, // words from lexicon not yet used
    c_words: Vec<Word>,   // vec of current words on screen / words that have been used
    score: i32,
    columns: u16,
    rows: u16,
    quit: bool,
    fps: Duration,
    fallspeed_cnt: i32,
    health: i32,
}

impl Game {
    fn new(so: Stdout, u_words: Vec<String>) -> Self {
        Self {
            so,
            input: String::new(),
            u_words,
            c_words: Vec::new(),
            score: 0,
            columns: 0,
            rows: 0,
            quit: false,
            fps: get_fps(FPS),
            fallspeed_cnt: 0,
            health: 3,
        }
    }

    fn game_loop(&mut self) -> io::Result<()> {
        if poll(Duration::ZERO)? {
            let event = read()?;

            match event {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match key.code {
                        KeyCode::Char(c) => {
                            self.input.push(c);
                        }
                        KeyCode::Backspace => {
                            self.input.pop();
                        }
                        KeyCode::Enter => {
                            self.check_validity_of_input();
                            self.input.clear();
                        }
                        KeyCode::Esc => {
                            self.quit = true;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // make words fall - if a word has fallen to far it gets removed
        if self.fallspeed_cnt >= FALLING_SPD {
            let mut index: Option<usize> = None;
            for (i, word) in self.c_words.iter_mut().enumerate() {
                if word.pos.1 == self.rows - 3 {
                    index = Some(i);
                } else {
                    word.pos.1 += 1;
                }
            }
            self.fallspeed_cnt = 0;
            if let Some(i) = index {
                self.c_words.remove(i);
                self.health -= 1;
            }
            self.clear_words()?;
            self.write_words()?;
        } else {
            self.fallspeed_cnt += 1;
            self.clear_words()?;
            self.write_words()?;
        }

        if self.c_words.len() < MAX_WORDS_IN_FRAME {
            self.gen_word();
        }

        self.write_ui()?;

        // quit conditionals
        if self.health <= 0 {
            self.quit(true)?;
        };
        if self.quit {
            self.quit(false)?;
        }

        // print queue
        self.so.flush()?;
        Ok(())
    }

    fn gen_word(&mut self) {
        let mut rng = rand::rng();

        let rand_word_i = rng.random_range(0..self.u_words.len());
        let rand_word = &self.u_words[rand_word_i];

        let word_len = rand_word.chars().count() as u16;
        let max_col = if self.columns > word_len {
            self.columns - word_len
        } else {
            0
        };

        let mut attempts = 0;
        let mut rand_col;
        'outer: loop {
            attempts += 1;
            if attempts > 100 {
                rand_col = 0;
                break;
            }

            rand_col = rng.random_range(0..=max_col);

            for word in &self.c_words {
                let word_start = word.pos.0;
                let word_end = word.pos.0 + word.text.chars().count() as u16;

                let new_start = rand_col;
                let new_end = rand_col + word_len;

                if !(new_end <= word_start || new_start >= word_end) {
                    continue 'outer;
                }
            }

            break;
        }

        self.c_words.push(Word {
            text: rand_word.to_string(),
            pos: (rand_col, 0),
        });
    }

    fn write_words(&mut self) -> io::Result<()> {
        for word in self.c_words.iter() {
            self.so.queue(cursor::MoveTo(word.pos.0, word.pos.1))?;
            self.so.write(word.text.as_bytes())?;
        }
        Ok(())
    }

    fn write_box(&mut self, content: String, w: u16, col: u16, row: u16) -> io::Result<()> {
        let bc = vec!["╭", "─", "╮", "│", "╯", "─", "╰", "│"];
        let top = format!("{}{}{}", bc[0], bc[1].repeat(w as usize - 2), bc[2]);
        let bot = format!("{}{}{}", bc[6], bc[1].repeat(w as usize - 2), bc[4]);
        let mid = format!(
            "{}{}{}{}",
            bc[3],
            content,
            " ".repeat(w as usize - 2 - content.chars().count()),
            bc[3]
        );
        self.so.queue(cursor::MoveTo(col, row - 2))?;
        self.so.write(top.as_bytes())?;
        self.so.queue(cursor::MoveTo(col, row - 1))?;
        self.so.write(mid.as_bytes())?;
        self.so.queue(cursor::MoveTo(col, row))?;
        self.so.write(bot.as_bytes())?;
        Ok(())
    }

    fn write_ui(&mut self) -> io::Result<()> {
        for i in 0..=3 {
            self.so.queue(cursor::MoveTo(0, self.rows - i))?;
            self.so.queue(Clear(ClearType::CurrentLine))?;
        }
        let prompt = self.input.clone();
        let score = format!("Score: {}", self.score);
        let health = format!(
            "Health: {}",
            (format!("{HEALTH_CHAR} ")).repeat(self.health as usize)
        );
        let total_rows = self.rows;
        let total_cols = self.columns;

        // prompt
        self.write_box(prompt, total_cols / 2, 0, total_rows - 3)?;
        // score
        self.write_box(score, total_cols / 4, total_cols / 2, total_rows - 3)?;
        // health
        self.write_box(
            health,
            total_cols / 4,
            (total_cols / 2) + (total_cols / 4),
            total_rows - 3,
        )?;

        Ok(())
    }

    /// if the user input matches one of the falling words:
    /// delete that word from the self.c_words vector
    /// and add a point to the score
    fn check_validity_of_input(&mut self) {
        let mut index: Option<usize> = None;
        for (i, word) in self.c_words.iter().enumerate() {
            if self.input.to_lowercase().trim() == word.text.trim() {
                self.score += 1;
                index = Some(i);
            }
        }
        if let Some(i) = index {
            self.c_words.remove(i);
        }
    }

    fn clear_words(&mut self) -> io::Result<()> {
        for row in 0..=(self.rows - 3) {
            self.so.queue(cursor::MoveTo(0, row))?;
            self.so.write(" ".repeat(self.columns as usize).as_bytes())?;
        }
        Ok(())
    }

    fn clear_screen(&mut self) {
        execute!(self.so, Clear(ClearType::All)).unwrap();
    }

    fn setup(&mut self) -> io::Result<()> {
        (self.columns, self.rows) = terminal::size()?;
        self.so.execute(cursor::Hide)?;
        self.clear_screen();
        Ok(())
    }

    fn quit(&mut self, user_lost: bool) -> io::Result<()> {
        self.clear_screen();
        let text_l1;
        match user_lost {
            true => text_l1 = format!("You lost all your health!"),
            false => text_l1 = format!("Goodbye!"),
        };
        let text_l2 = format!("Your final score: {}", self.score);
        self.wr_ce_txt(text_l1, 0)?;
        self.wr_ce_txt(text_l2, 1)?;
        self.so.flush()?;
        thread::sleep(Duration::from_secs(3));
        self.so.queue(cursor::MoveTo(0, 0))?;
        self.so.execute(cursor::Show)?;
        self.clear_screen();
        Ok(())
    }

    fn wr_ce_txt(&mut self, text: String, offset: u16) -> io::Result<()> {
        self.so.queue(cursor::MoveTo(
            (self.columns / 2) - (text.len() as u16 / 2),
            self.rows / 2 + offset,
        ))?;
        self.so.write(text.as_bytes())?;
        Ok(())
    }

    fn intro(&mut self) -> io::Result<()> {
        self.clear_screen();
        let text_1 = format!("Type the falling words as fast as you can!");
        let text_2 =
        format!("Get as many points as possible without depleting your health!");
        self.wr_ce_txt(text_1, 0)?;
        self.wr_ce_txt(text_2, 1)?;
        self.so.flush()?;
        thread::sleep(Duration::from_secs(3));
        self.clear_screen();
        Ok(())
    }
}

fn get_fps(fps: f64) -> Duration {
    Duration::from_secs_f64(1.0 / fps)
}
