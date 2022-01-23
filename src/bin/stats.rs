use std::cmp::Ordering;

use wordle::*;

use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;

#[derive(PartialEq)]
struct NonNan(f64);

impl PartialOrd for NonNan {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Eq for NonNan {
    fn assert_receiver_is_total_eq(&self) {}
}

impl Ord for NonNan {
    fn cmp(&self, other: &NonNan) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

fn main() {
    let mut answers = include_str!("../../words/wordle-answers.txt")
        .lines()
        .collect::<Vec<_>>();

    let all_words = answers
        .iter()
        .cloned()
        .chain(include_str!("../../words/wordle-allowed-guesses.txt").lines())
        .collect::<Vec<_>>();

    let mut average_words_after_guess = vec![0_f64; all_words.len()];

    // tweak this to get an approximate ranking a little faster
    let test_size: usize = answers.len();
    answers.shuffle(&mut thread_rng());

    for answer in answers.iter().take(test_size) {
        println!("testing '{}' as the wordle", answer);
        let wordle = Wordle::new(answer);
        let solver = Solver::new(wordle);
        let words_after_guesses = all_words
            .par_iter()
            .map(|guess| solver.num_filtered_words(guess, &answers) as f64 / test_size as f64)
            .collect::<Vec<_>>();

        average_words_after_guess
            .iter_mut()
            .zip(words_after_guesses)
            .for_each(|(sum, new)| *sum += new);
    }

    let mut words_and_scores = all_words
        .iter()
        .zip(average_words_after_guess)
        .collect::<Vec<_>>();
    words_and_scores.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());

    println!("\n{}\n", "#".repeat(80));
    for (i, (guess, n)) in words_and_scores.iter().enumerate() {
        println!(
            "guess #{}: {} ({:.2} average consistent words afterwards)",
            i + 1,
            guess,
            n
        );
    }
}
