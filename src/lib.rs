const INCORRECT: u8 = 0;
const CORRECT_CHAR: u8 = 2;
const CORRECT: u8 = 3;

pub const WORD_LEN: usize = 5;

#[derive(Clone)]
pub struct Wordle<'a> {
    word: &'a str,
}

impl<'a> Wordle<'a> {
    pub fn new(word: &str) -> Wordle {
        Wordle { word }
    }

    pub fn check(&self, guess: &str) -> [u8; WORD_LEN] {
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

pub struct WordPattern {
    mask: [Option<char>; WORD_LEN],
    required_chars: Vec<char>,
    bad_chars_by_pos: [Vec<char>; WORD_LEN],
}

impl WordPattern {
    pub fn new(word: &str, result: [u8; WORD_LEN]) -> WordPattern {
        let mut mask = [None; WORD_LEN];
        let mut required_chars = vec![];
        let mut bad_chars_by_pos = <[Vec<char>; WORD_LEN]>::default();
        for (i, res) in result.into_iter().enumerate() {
            let char = word.chars().nth(i).unwrap();
            match res {
                CORRECT => {
                    mask[i] = Some(char);
                }
                CORRECT_CHAR => {
                    required_chars.push(char);
                    bad_chars_by_pos[i].push(char);
                }
                _ => {
                    bad_chars_by_pos.iter_mut().for_each(|v| {
                        v.push(char);
                    });
                }
            }
        }

        WordPattern {
            mask,
            required_chars,
            bad_chars_by_pos,
        }
    }

    pub fn matches(&self, word: &str) -> bool {
        word.chars().enumerate().all(|(i, c)| {
            self.mask[i] == Some(c)
                || self.mask[i].is_none() && !self.bad_chars_by_pos[i].contains(&c)
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

    pub fn filter_words(&mut self, word: &str, words: &mut Vec<&'a str>) {
        let filter = WordPattern::new(word, self.wordle.check(word));

        let mut i = 0;
        while i < words.len() {
            let word = words[i];
            if !filter.matches(word) {
                words.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn num_filtered_words(&self, word: &str, words: &[&'a str]) -> usize {
        let filter = WordPattern::new(word, self.wordle.check(word));
        words.iter().filter(|w| filter.matches(w)).count()
    }
}
