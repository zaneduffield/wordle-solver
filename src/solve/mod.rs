use std::io::Stdout;

use std::io::stdout;

use crossterm::cursor;
use crossterm::style::Color;
use crossterm::style::Stylize;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    style::{self},
};

use crate::ui::*;
use wordle::*;

fn select(stdout: &mut Stdout, c: &mut WordleChar) -> crossterm::Result<()> {
    draw_border(stdout, c)?;
    update_char(stdout, c, true)?;
    Ok(())
}

fn deselect(stdout: &mut Stdout, c: &mut WordleChar) -> crossterm::Result<()> {
    draw_border(stdout, c)?;
    update_char(stdout, c, false)?;
    Ok(())
}

fn left(stdout: &mut Stdout, word: &mut Vec<WordleChar>, i: &mut usize) -> crossterm::Result<()> {
    if *i > 0 {
        deselect(stdout, &mut word[*i])?;
        *i -= 1;
        select(stdout, &mut word[*i])?;
    }
    Ok(())
}

fn right(stdout: &mut Stdout, word: &mut Vec<WordleChar>, i: &mut usize) -> crossterm::Result<()> {
    if *i < word.len() - 1 {
        deselect(stdout, &mut word[*i])?;
        *i += 1;
        select(stdout, &mut word[*i])?;
    }
    Ok(())
}

fn up(stdout: &mut Stdout, c: &mut WordleChar) -> crossterm::Result<()> {
    c.result = match c.result {
        CharResult::Correct => CharResult::Incorrect,
        CharResult::CorrectChar => CharResult::Correct,
        CharResult::Incorrect => CharResult::CorrectChar,
    };
    update_char(stdout, c, true)?;
    Ok(())
}

fn down(stdout: &mut Stdout, c: &mut WordleChar) -> crossterm::Result<()> {
    c.result = match c.result {
        CharResult::Correct => CharResult::CorrectChar,
        CharResult::CorrectChar => CharResult::Incorrect,
        CharResult::Incorrect => CharResult::Correct,
    };
    update_char(stdout, c, true)?;
    Ok(())
}

fn poll_guess_result(
    stdout: &mut Stdout,
    guess: &str,
    col: u16,
    row: &mut u16,
) -> Result<PollResult<GameControl>, crossterm::ErrorKind> {
    let mut word = draw_word(stdout, guess, col, row)?;
    let first = &mut word[0];
    let mut i = 0;
    select(stdout, first)?;
    loop {
        match event::read()? {
            Event::Key(event) => match event.code {
                KeyCode::Left => left(stdout, &mut word, &mut i)?,
                KeyCode::Right => right(stdout, &mut word, &mut i)?,
                KeyCode::Up => up(stdout, &mut word[i])?,
                KeyCode::Down => down(stdout, &mut word[i])?,
                KeyCode::Enter => break,
                _ if is_quit_event(event) => return Ok(PollResult::Control(GameControl::Quit)),
                _ if is_restart_event(event) => {
                    return Ok(PollResult::Control(GameControl::Restart))
                }
                _ => {}
            },
            _ => continue,
        };
    }
    deselect(stdout, &mut word[i])?;

    let mut out = [CharResult::Incorrect; WORD_LEN];
    word.iter()
        .zip(out.iter_mut())
        .for_each(|(c, o)| *o = c.result);
    Ok(PollResult::Score(WordPattern::new(guess, out)))
}

fn print_intro(stdout: &mut Stdout) -> crossterm::Result<()> {
    execute!(
        stdout,
        style::Print("
Let's play Wordle! You think of a word, and mark my guesses.
\r\nUse the arrow keys to set the colour of each character in the guess, and press enter to confirm.\n"),
)?;
    print_controls_explanation(stdout)?;
    print_colour_explanation(stdout)?;
    execute!(stdout, style::Print("\n"))?;

    Ok(())
}

fn print_no_answer_msg(stdout: &mut Stdout) -> crossterm::Result<()> {
    execute!(
        stdout,
        style::PrintStyledContent(
            "Something went wrong; I couldn't find an answer.\n".with(Color::Red)
        )
    )
}

fn solve_with<F>(mut guess_poller: F) -> crossterm::Result<()>
where
    F: FnMut(
        &mut Stdout,
        &str,
        u16,
        &mut u16,
    ) -> Result<PollResult<GameControl>, crossterm::ErrorKind>,
{
    let stdout = &mut stdout();
    'game: loop {
        init(stdout)?;
        print_intro(stdout)?;

        let answers = answers();
        let guesses = guesses();

        let (col, mut row) = (0, cursor::position()?.1);
        let result = run_solver(first_guess(), answers.clone(), guesses.clone(), |guess| {
            guess_poller(stdout, guess, col, &mut row)
        });
        execute!(stdout, style::Print("\r\n\n"))?;
        match result? {
            SolverResult::NoAnswer => print_no_answer_msg(stdout)?,
            SolverResult::Answer {
                word: _,
                guess_count,
            } => print_solved_msg(stdout, guess_count)?,
            SolverResult::Control(GameControl::Restart) => continue 'game,
            SolverResult::Control(GameControl::Quit) => break 'game,
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

pub fn solve() -> crossterm::Result<()> {
    let poller = |stdout: &mut Stdout, guess: &str, col: u16, row: &mut u16| {
        poll_guess_result(stdout, guess, col, row)
    };
    solve_with(poller)
}

pub fn solve_for_answer(answer: &str) -> crossterm::Result<()> {
    let wordle = Wordle::new(answer);
    let poller = |stdout: &mut Stdout, guess: &str, col: u16, row: &mut u16| {
        let result = wordle.check(guess);
        let mut word = draw_word(stdout, guess, col, row)?;
        score_guess(stdout, &mut word, result)?;

        Ok(PollResult::Score(WordPattern::new(guess, result)))
    };
    solve_with(poller)
}
