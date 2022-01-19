use std::{
    env,
    io::{self, Write},
};

use wordle::*;

fn main() {
    let mut answers = include_str!("../../words/wordle-answers.txt")
        .lines()
        .collect::<Vec<_>>();

    let word = env::args()
        .nth(1)
        .expect("first argument should be the wordle for the day");
    let wordle = Wordle::new(&word);

    let mut solver = Solver::new(wordle);
    let mut line = String::new();
    loop {
        print!("make a guess: ");
        io::stdout().flush().unwrap();
        if matches!(io::stdin().read_line(&mut line), Err(_)) {
            break;
        } else if line.trim_end().len() != WORD_LEN {
            println!("guess must be of length {}", WORD_LEN);
        } else {
            solver.filter_words(&line, &mut answers);
            println!("{} consistent words remaining", answers.len());
            if answers.len() < 10 {
                println!("consistent words: {:?}", answers);
            }
        }
        line.clear();
    }
}
