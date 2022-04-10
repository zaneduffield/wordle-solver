use itertools::Itertools;
use std::collections::BTreeMap;
use std::{cmp::Reverse, time::Instant};

use rustc_hash::FxHashSet;
use serde::Deserialize;
use serde::Serialize;

use wordle::*;
use crate::solve::*;

const RESULTS: [CharResult; 3] = [
    CharResult::Incorrect,
    CharResult::CorrectChar,
    CharResult::Correct,
];

#[derive(Serialize, Deserialize)]
struct BestGuess {
    init_guess: String,
    result: GuessResult,
    best_guess: String,
}

#[derive(Serialize, Deserialize)]
struct Precompute {
    data: Vec<BestGuess>,
}

pub fn precompute() {
    let mut perms = FxHashSet::default();
    for comb in RESULTS.into_iter().combinations_with_replacement(WORD_LEN) {
        comb.iter().cloned().permutations(WORD_LEN).for_each(|x| {
            perms.insert(x);
        });
    }

    let first_guess = first_guess();
    let guesses = guesses();

    let mut durs = vec![];

    for perm in &perms {
        if perm.iter().filter(|r| r == &&CharResult::Correct).count() > 2 {
            continue;
        }
        eprintln!("testing {:?}", perm);
        let mut answers = answers();

        let start = Instant::now();

        let res = [perm[0], perm[1], perm[2], perm[3], perm[4]];
        let pat = WordPattern::new(first_guess, res);
        filter_words(&pat, &mut answers);
        let best = match best_guess(&guesses, &answers) {
            Some(x) => x,
            None => continue,
        };

        let dur = Instant::now() - start;

        durs.push((dur, res, best));
    }

    let len = durs.len();
    let data = Precompute {
        data: durs
            .into_iter()
            .sorted_by_key(|(d, _, _)| Reverse(*d))
            .map(|(_, result, best)| BestGuess {
                init_guess: first_guess.to_string(),
                result,
                best_guess: best.to_string(),
            })
            .take(len / 2)
            .collect_vec(),
    };

    println!("{}", serde_json::to_string(&data).unwrap());
}

pub fn parse_precomputed_data() -> BTreeMap<(String, GuessResult), String> {
    serde_json::from_str::<Precompute>(include_str!("precompute.json"))
        .unwrap()
        .data
        .into_iter()
        .map(|b| ((b.init_guess, b.result), b.best_guess))
        .collect()
}
