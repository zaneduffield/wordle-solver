use std::fmt::Display;

use rayon::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CharResult {
    Unknown,
    Incorrect,
    CorrectChar,
    Correct,
}

pub const WORD_LEN: usize = 5;

pub type GuessResult = [CharResult; WORD_LEN];
#[derive(Clone)]
pub struct Wordle<'a> {
    word: &'a str,
}

impl<'a> Wordle<'a> {
    pub fn new(word: &str) -> Wordle {
        Wordle { word }
    }

    pub fn check(&self, guess: &str) -> GuessResult {
        /* we need to loop through the chars twice because wordle will not mark
        a character as 'wrong position' when it was marked as correct elsewhere */
        let mut out = [CharResult::Incorrect; WORD_LEN];

        // first mark all the correct chars, and store all chars that were missed
        let mut missed_chars = ['\0'; WORD_LEN];
        let mut count = 0;
        for ((g, w), o) in guess.chars().zip(self.word.chars()).zip(out.iter_mut()) {
            if g == w {
                *o = CharResult::Correct;
            } else {
                missed_chars[count] = w;
                count += 1;
            }
        }

        // mark all guess chars that were 'missed'
        guess
            .chars()
            .zip(out.iter_mut())
            .filter(|(g, o)| !matches!(o, CharResult::Correct) && missed_chars.contains(g))
            .for_each(|(_, o)| *o = CharResult::CorrectChar);

        out
    }
}

pub struct WordPattern {
    mask: [Option<char>; WORD_LEN],
    required_chars: Vec<char>,
    bad_chars_by_pos: [Vec<char>; WORD_LEN],
    bad_chars: Vec<char>,
}

impl WordPattern {
    pub fn new(word: &str, result: GuessResult) -> WordPattern {
        let mut mask = [None; WORD_LEN];
        let mut required_chars = vec![];
        let mut bad_chars_by_pos = <[Vec<char>; WORD_LEN]>::default();
        let mut bad_chars = vec![];
        for (i, res) in result.into_iter().enumerate() {
            let char = word.chars().nth(i).unwrap();
            match res {
                CharResult::Correct => {
                    mask[i] = Some(char);
                }
                CharResult::CorrectChar => {
                    required_chars.push(char);
                    bad_chars_by_pos[i].push(char);
                }
                CharResult::Incorrect => {
                    bad_chars.push(char);
                    bad_chars_by_pos[i].push(char);
                }
                CharResult::Unknown => {}
            }
        }

        let mut i = 0;
        while i < bad_chars.len() {
            let c = bad_chars[i];
            if mask.iter().any(|&m| m == Some(c)) {
                bad_chars.swap_remove(i);
            } else {
                i += 1;
            }
        }

        WordPattern {
            mask,
            required_chars,
            bad_chars_by_pos,
            bad_chars,
        }
    }

    pub fn is_perfect_match(&self) -> bool {
        self.mask.iter().all(|m| m.is_some())
    }

    pub fn matches(&self, word: &str) -> bool {
        word.chars().enumerate().all(|(i, c)| {
            self.mask[i] == Some(c)
                || self.mask[i].is_none()
                    && !self.bad_chars_by_pos[i].contains(&c)
                    && !self.bad_chars.contains(&c)
        }) && self
            .required_chars
            .iter()
            .all(|&c1| word.chars().any(|c2| c1 == c2))
    }
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

pub fn guesses() -> Vec<&'static str> {
    include_str!("../words/wordle-allowed-guesses.txt")
        .lines()
        .collect::<Vec<_>>()
}

pub fn answers() -> Vec<&'static str> {
    include_str!("../words/wordle-answers.txt")
        .lines()
        .collect::<Vec<_>>()
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

pub fn first_guess<'a>() -> &'a str {
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
            guess = match best_guess(&guesses, &answers) {
                None => return Ok(SolverResult::NoAnswer),
                Some(x) => x,
            };
        }
    }
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
