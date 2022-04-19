use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Serialize, Deserialize)]
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
    pub result: GuessResult,
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
            result,
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
