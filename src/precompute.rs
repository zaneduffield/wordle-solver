use std::{time::Instant, cmp::Reverse};
use itertools::Itertools;

use rustc_hash::{FxHashSet, FxHashMap};
use wordle::*;

const RESULTS: [CharResult; 3] = [CharResult::Incorrect, CharResult::CorrectChar, CharResult::Correct];

fn precompute() -> FxHashMap<(GuessResult, String), String> {
    let mut perms = FxHashSet::default();
    for comb in RESULTS.into_iter().combinations_with_replacement(WORD_LEN) {
        comb.iter().cloned().permutations(WORD_LEN).for_each(|x| {perms.insert(x);});
    }

    let first_guess = first_guess();
    let guesses = guesses();

    let mut durs = vec![];

    for perm in &perms {
        if perm.iter().filter(|r| r == &&CharResult::Correct).count() >= 2 {
            continue;
        }
        println!("testing {:?}", perm);
        let mut answers = answers();

        let start = Instant::now();

        let res = [perm[0], perm[1], perm[2], perm[3], perm[4]];
        let pat = WordPattern::new(first_guess, res);
        filter_words(&pat, &mut answers);
        let best = match best_guess(&guesses, &answers) {
            Some(x) => x,
            None => continue,
        };

        let end = Instant::now();

        durs.push((end - start, res, best));
    }

    durs.sort_by_key(|(d, _, _)| Reverse(*d));
    durs.into_iter().map(|(_, r, g)| ((r, first_guess.to_string()), g.to_string())).collect()
}

fn main() {
    precompute();
}