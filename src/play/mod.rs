use std::io::Stdout;

use std::io::stdout;

use crossterm::cursor;
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