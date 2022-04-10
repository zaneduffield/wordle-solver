use std::io::Stdout;

use std::io::stdout;
use std::io::Write;

use crossterm::cursor;
use crossterm::queue;
use crossterm::style::Attribute;
use crossterm::style::Color;
use crossterm::style::Stylize;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    style::{self},
    terminal::{self},
};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use rustc_hash::FxHashSet;

use crate::ui::*;
use wordle::*;

const ERR_ROW_OFFSET: u16 = 1;
const ERR_COL: u16 = 2;

fn display_err_line(stdout: &mut Stdout, msg: &str) -> crossterm::Result<()> {
    let (col, row) = cursor::position()?;
    execute!(
        stdout,
        cursor::MoveTo(ERR_COL, row + ERR_ROW_OFFSET),
        style::PrintStyledContent(msg.with(Color::Red)),
        cursor::MoveTo(col, row),
    )
}

fn clear_err_line(stdout: &mut Stdout) -> crossterm::Result<()> {
    let (col, row) = cursor::position()?;
    execute!(
        stdout,
        cursor::MoveTo(ERR_COL, row + ERR_ROW_OFFSET),
        terminal::Clear(terminal::ClearType::UntilNewLine),
        cursor::MoveTo(col, row),
    )
}

enum PollResult {
    Guess(String),
    Control(GameControl),
}

fn poll_guess(
    stdout: &mut Stdout,
    word: &mut Vec<WordleChar>,
    valid_guesses: &FxHashSet<&str>,
) -> crossterm::Result<PollResult> {
    let mut i = 0;
    let mut guess = String::new();
    loop {
        if let Event::Key(event) = event::read()? {
            match event.code {
                _ if is_quit_event(event) => return Ok(PollResult::Control(GameControl::Quit)),
                _ if is_restart_event(event) => {
                    return Ok(PollResult::Control(GameControl::Restart))
                }
                KeyCode::Char(c) => {
                    if i < word.len() {
                        word[i].c = c.to_ascii_lowercase();
                        update_char(stdout, &word[i], false)?;
                        i += 1;
                    }
                }
                KeyCode::Backspace | KeyCode::Delete => {
                    i = if i == 0 { 0 } else { i - 1 };
                    word[i].c = ' ';
                    update_char(stdout, &word[i], false)?;
                    clear_err_line(stdout)?;
                }
                KeyCode::Enter => {
                    if i == word.len() {
                        guess.clear();
                        guess.extend(word.iter().map(|c| c.c));
                        if valid_guesses.contains(guess.as_str()) {
                            clear_err_line(stdout)?;
                            return Ok(PollResult::Guess(guess));
                        } else {
                            display_err_line(stdout, "not in the word list")?;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn print_intro(stdout: &mut Stdout) -> crossterm::Result<()> {
    execute!(
        stdout,
        style::Print("\nLet's play Wordle! Just type your guesses and hit enter to confirm.\n\n"),
    )?;
    print_controls_explanation(stdout)?;
    print_colour_explanation(stdout)?;
    execute!(stdout, style::Print("\n"))?;

    Ok(())
}

struct Keyb {
    rows: [(String, Vec<(char, CharResult)>); 3],
}

impl Keyb {
    fn new() -> Keyb {
        let make_row = |chars: &str| chars.chars().map(|c| (c, CharResult::Unknown)).collect();
        let top = "QWERTYUIOP";
        let mid = "ASDFGHJKL";
        let bot = "ZXCVBNM";
        Keyb {
            rows: [
                ("     ".to_string(), make_row(top)),
                ("      ".to_string(), make_row(mid)),
                ("       ".to_string(), make_row(bot)),
            ],
        }
    }
}

fn print_keyb(stdout: &mut Stdout, keyb: &mut Keyb, row: u16, col: u16) -> crossterm::Result<()> {
    queue!(stdout, cursor::MoveTo(col, row))?;

    for (pad, row) in &keyb.rows {
        queue!(stdout, style::Print(pad))?;
        for (c, r) in row {
            let attr = if r == &CharResult::Incorrect {
                Attribute::Dim
            } else {
                Attribute::Bold
            };
            let styled_c = c.with(result_col(*r)).attribute(attr);
            queue!(
                stdout,
                style::PrintStyledContent(styled_c),
                style::Print(" ")
            )?;
        }
        queue!(stdout, style::Print("\r\n"))?;
    }
    queue!(stdout, style::Print("\r\n"))?;
    stdout.flush()?;

    Ok(())
}

fn update_keyb(keyb: &mut Keyb, guess: &str, score: GuessResult) {
    for (_, row) in &mut keyb.rows {
        for (gc, gr) in guess.chars().zip(score) {
            row.iter_mut()
                .filter(|(c, r)| gc.eq_ignore_ascii_case(c) && *r != CharResult::Correct)
                .for_each(|(_, r)| *r = gr);
        }
    }
}

pub fn play(answer: Option<&String>) -> crossterm::Result<()> {
    let answer = answer.map(|s| s.as_str());

    let stdout = &mut stdout();
    let answers = answers();
    let valid_guesses = guesses()
        .into_iter()
        .chain(answers.clone())
        .collect::<FxHashSet<_>>();

    'game: loop {
        init(stdout)?;
        print_intro(stdout)?;
        let mut keyb = Keyb::new();
        let (keyb_col, keyb_row) = (0, cursor::position()?.1);
        print_keyb(stdout, &mut keyb, keyb_row, keyb_col)?;

        let word = answer.unwrap_or_else(|| answers.choose(&mut thread_rng()).unwrap());
        let wordle = Wordle::new(word);

        let (col, mut row) = (0, cursor::position()?.1);
        let mut guess_count = 0;
        'guess: loop {
            let mut word = draw_word(stdout, &" ".repeat(WORD_LEN), col, &mut row)?;
            let guess = match poll_guess(stdout, &mut word, &valid_guesses)? {
                PollResult::Control(GameControl::Quit) => break 'game,
                PollResult::Control(GameControl::Restart) => continue 'game,
                PollResult::Guess(guess) => guess,
            };
            guess_count += 1;

            let score = wordle.check(&guess);
            score_guess(stdout, &mut word, score)?;

            execute!(stdout, cursor::SavePosition)?;
            update_keyb(&mut keyb, &guess, score);
            print_keyb(stdout, &mut keyb, keyb_row, keyb_col)?;
            execute!(stdout, cursor::RestorePosition)?;

            if score.iter().all(|r| matches!(r, CharResult::Correct)) {
                execute!(stdout, style::Print("\r\n\n"))?;
                print_solved_msg(stdout, guess_count)?;
                break 'guess;
            }
        }
        print_controls_explanation(stdout)?;
        'gameover: loop {
            match event::read()? {
                Event::Key(e) if is_quit_event(e) => break 'game,
                Event::Key(e) if is_restart_event(e) => continue 'game,
                _ => continue 'gameover,
            }
        }
    }

    fini(stdout)?;
    Ok(())
}
