use std::env;

use rustc_hash::FxHashSet;

const INCORRECT: u8 = 0;
const CORRECT_CHAR: u8 = 2;
const CORRECT: u8 = 3;

const WORD_LEN: usize = 5;

#[derive(Clone)]
struct Wordle<'a> {
    word: &'a str,
}

impl<'a> Wordle<'a> {
    fn check(&self, guess: &str) -> [u8; WORD_LEN] {
        let mut out = [INCORRECT; WORD_LEN];
        for (i, c) in guess.chars().enumerate() {
            if self.word.chars().nth(i) == Some(c) {
                out[i] = CORRECT;
            } else if self.word.chars().any(|c2| c == c2) {
                out[i] = CORRECT_CHAR;
            }
        }

        out
    }
}

#[derive(Clone)]
struct Solver<'a> {
    wordle: Wordle<'a>,
    mask: [Option<char>; WORD_LEN],
    words: Vec<&'a str>,
    bad_chars_by_pos: [FxHashSet<char>; WORD_LEN],
    good_chars: FxHashSet<char>,
}

impl<'a> Solver<'a> {
    fn new(wordle: Wordle<'a>, words: Vec<&'a str>) -> Solver<'a> {
        Solver {
            wordle,
            mask: [None; WORD_LEN],
            words,
            bad_chars_by_pos: <[FxHashSet<char>; WORD_LEN]>::default(),
            good_chars: FxHashSet::default(),
        }
    }

    fn guess(&mut self, word: &str) {
        let result = self.wordle.check(word);
        for (i, res) in result.into_iter().enumerate() {
            let char = word.chars().nth(i).unwrap();
            match res {
                CORRECT => {
                    self.mask[i] = Some(char);
                }
                CORRECT_CHAR => {
                    self.good_chars.insert(char);
                    self.bad_chars_by_pos[i].insert(char);
                }
                _ => {
                    self.bad_chars_by_pos.iter_mut().for_each(|v| {
                        v.insert(char);
                    });
                }
            }
        }

        let mut i = 0;
        while i < self.words.len() {
            let word = self.words[i];
            if !self.matches(word) {
                self.words.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn matches(&self, word: &str) -> bool {
        word.chars().enumerate().all(|(i, c)| {
            self.mask[i] == Some(c)
                || self.mask[i].is_none() && !self.bad_chars_by_pos[i].contains(&c)
        }) && self
            .good_chars
            .iter()
            .all(|&c1| word.chars().any(|c2| c1 == c2))
    }
}

fn main() {
    let mut words = include_str!("../words/wordle-answers.txt")
        .lines()
        .collect::<Vec<_>>();
    let mut guesses = include_str!("../words/wordle-allowed-guesses.txt")
        .lines()
        .collect::<Vec<_>>();
    let all_words = words.into_iter().chain(guesses.into_iter()).collect::<Vec<_>>();

    let word = env::args().nth(1).unwrap();
    let wordle = Wordle { word: &word };

    let solver = Solver::new(wordle, all_words);
    while solver.words.len() > 1 {
        // find best guess and then apply it
        for guess in &solver.words {
            let mut local_solver = solver.clone();
        }
    }
}
