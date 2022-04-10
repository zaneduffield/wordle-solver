use std::fmt::Display;
use std::io::Stdout;

use rayon::prelude::*;
use std::io::stdout;

use crossterm::cursor;
use crossterm::style::Color;
use crossterm::style::Stylize;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    style::{self},
};

use crate::precompute::*;
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
        CharResult::Unknown => CharResult::CorrectChar,
    };
    update_char(stdout, c, true)?;
    Ok(())
}

fn down(stdout: &mut Stdout, c: &mut WordleChar) -> crossterm::Result<()> {
    c.result = match c.result {
        CharResult::Correct => CharResult::CorrectChar,
        CharResult::CorrectChar => CharResult::Incorrect,
        CharResult::Incorrect => CharResult::Correct,
        CharResult::Unknown => CharResult::Correct,
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

#[derive(Clone)]
pub struct Solver<'a> {
    wordle: Wordle<'a>,
}

impl<'a> Solver<'a> {
    pub fn new(wordle: Wordle<'a>) -> Solver<'a> {
        Solver { wordle }
    }

    pub fn num_filtered_words(&self, word: &str, words: &[&'a str]) -> usize {
        let filter = WordPattern::new(word, self.wordle.check(word));
        words.iter().filter(|w| filter.matches(w)).count()
    }
}

pub fn filter_words(pattern: &WordPattern, words: &mut Vec<&str>) {
    let mut i = 0;
    while i < words.len() {
        let word = words[i];
        if !pattern.matches(word) {
            words.swap_remove(i);
        } else {
            i += 1;
        }
    }
}

pub fn best_guess<'a>(guess_pool: &[&'a str], answer_pool: &[&'a str]) -> Option<&'a str> {
    /*
        We want the word with the minimal positive maximum number of filtered words.
        This is the word with the best worst-case among all the possible answers.
        It needs to be positive because a zero would mean that in every single case
        the guess leads to no valid words - a dead end.

        Note that we put the 'answer_pool' first in the iterator chain so that if two
        words are equally 'good' we will pick the one which could actually be the
        answer.

        A parallel `min_by_key` algorithm doesn't guarantee that earlier elements
        will be favoured in the case of ties, which is why we create an enumerated
        iterator and find the minimum of the tuple with the index.
    */
    match answer_pool.len() {
        0 => None,
        1 => Some(answer_pool[0]),
        _ => answer_pool
            .par_iter()
            .chain(guess_pool)
            .enumerate()
            .min_by_key(|&(i, guess)| {
                (
                    match answer_pool
                        .iter()
                        .map(|a| Solver::new(Wordle::new(a)))
                        .map(|s| s.num_filtered_words(guess, answer_pool))
                        .max()
                    {
                        None | Some(0) => usize::MAX,
                        Some(x) => x,
                    },
                    i,
                )
            })
            .map(|(_, x)| *x),
    }
}

pub const fn first_guess<'a>() -> &'a str {
    // other good first guesses include:
    //   "roate", "raile", "arise", "irate", "orate", "ariel", "raine"
    "trace"
}

pub enum PollResult<Control> {
    Control(Control),
    Score(WordPattern),
}

pub enum SolverResult<Control> {
    Control(Control),
    Answer { word: String, guess_count: u32 },
    NoAnswer,
}

pub fn run_solver<'a, F, Err: Display, Control>(
    guess: &str,
    mut answers: Vec<&'a str>,
    guesses: Vec<&'a str>,
    mut poll_guess_result: F,
) -> Result<SolverResult<Control>, Err>
where
    F: FnMut(&str) -> Result<PollResult<Control>, Err>,
{
    let precomp = parse_precomputed_data();
    let mut guess_count = 0;
    let mut guess = guess;
    loop {
        guess_count += 1;
        let pattern = match poll_guess_result(guess)? {
            PollResult::Control(c) => return Ok(SolverResult::Control(c)),
            PollResult::Score(p) => p,
        };
        if pattern.is_perfect_match() {
            return Ok(SolverResult::Answer {
                word: guess.to_string(),
                guess_count,
            });
        } else {
            filter_words(&pattern, &mut answers);
            guess = match precomp.get(&(guess.to_string(), pattern.result)) {
                Some(guess) => guess,
                None => match best_guess(&guesses, &answers) {
                    None => return Ok(SolverResult::NoAnswer),
                    Some(x) => x,
                },
            };
            // guess = match best_guess(&guesses, &answers) {
            //     None => return Ok(SolverResult::NoAnswer),
            //     Some(x) => x,
            // };
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_solver(first_guess: &str, answers: &[&str], guesses: &[&str]) {
        let mut games = Vec::with_capacity(answers.len());
        for answer in answers {
            let wordle = Wordle::new(answer);
            let count = match run_solver::<_, std::io::Error, ()>(
                first_guess,
                answers.to_vec(),
                guesses.to_vec(),
                |guess| {
                    Ok(PollResult::Score(WordPattern::new(
                        guess,
                        wordle.check(guess),
                    )))
                },
            )
            .unwrap()
            {
                SolverResult::Answer {
                    word: _,
                    guess_count,
                } => guess_count,
                _ => u32::MAX,
            };

            games.push((count, answer));
        }

        games.sort();
        const MAX_GUESSES: u32 = 6;
        let slow_solves = games
            .iter()
            .filter(|(c, _)| *c > MAX_GUESSES)
            .collect::<Vec<_>>();
        println!(
            "\n{} words couldn't be solved in {} guesses, using {} as the first guess.",
            slow_solves.len(),
            MAX_GUESSES,
            first_guess,
        );
        if !slow_solves.is_empty() {
            println!("They are {:?}", slow_solves);
        }
    }
    #[test]
    fn test() {
        test_solver(first_guess(), &answers(), &guesses());
    }
}
