use crossterm::{
    ExecutableCommand, QueueableCommand, cursor,
    event::{Event, KeyCode, KeyEventKind, poll, read},
    execute,
    terminal::{self, Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use std::{
    io::{self, Stdout, Write, stdout},
    thread,
    time::Duration,
};

const FPS: f64 = 30.0;
const TIME_LIMIT: f64 = 30.0;
const LEXICON: &str = include_str!("lexicon.txt");

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
        game.so.flush()?;
    }

    game.quit()?;
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
    ui_printed: bool,
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
            ui_printed: false,
        }
    }

    fn game_loop(&mut self) -> io::Result<()> {
        if poll(Duration::from_millis(2))? {
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

        self.write_main()?;
        self.write_prompt()?;

        if self.quit {
            self.quit()?;
        };
        Ok(())
    }

    fn write_main(&mut self) -> io::Result<()> {
        todo!();
        //
        // Ok(())
    }

    fn write_prompt(&mut self) -> io::Result<()> {
        if !self.ui_printed {
            self.so.queue(cursor::MoveTo(0, self.rows - 2))?;
            let prompt_sep = "─".repeat(self.columns as usize);
            self.so.write(prompt_sep.as_bytes())?;
            self.ui_printed = true;
        }

        self.so.queue(cursor::MoveTo(0, self.rows - 1))?;
        self.so.queue(Clear(ClearType::CurrentLine))?;
        self.so.write(self.input.as_bytes())?;
        Ok(())
    }

    /// if the user input matches one of the falling words:
    /// delete that word from the self.c_words vector
    /// and add a point to the score
    fn check_validity_of_input(&mut self) {
        let mut index: Option<usize> = None;
        for (i, word) in self.c_words.iter().enumerate() {
            if self.input.to_lowercase() == word.text {
                self.score += 1;
                index = Some(i);
            }
        }
        if let Some(i) = index {
            self.c_words.remove(i);
        }
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

    fn quit(&mut self) -> io::Result<()> {
        self.clear_screen();
        let text = format!("Your final score: {}", self.score);
        self.wr_ce_txt(text, 0)?;
        self.so.flush()?;
        thread::sleep(Duration::from_secs(2));
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
        let text_2 = format!("Get as many points as possible within {TIME_LIMIT} seconds!");
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
