use pancurses::*;
use std::io::{Read, Write};
use wordle::*;

const CORRECT_COL_PAIR: i16 = 15;
const CORRECT_CHAR_COL_PAIR: i16 = 16;
const INCORRECT_COL_PAIR: i16 = 17;
const SELECTED_COL_PAIR: i16 = 18;
const UNSELECTED_COL_PAIR: i16 = 19;

const SIDE_BUFF: usize = 2;
const BETWEEN_GUESS_BUFF: usize = 0;
const CELL_W: usize = 3 + SIDE_BUFF;
const CELL_H: usize = 3;
const CELL_BUFF: i32 = 0;
const CHAR_POS_Y: i32 = 1;
const CHAR_POS_X: i32 = 2;

struct WordleChar {
    c: char,
    window: Window,
    result: CharResult,
}

fn init() -> Window {
    let w = initscr();
    noecho();
    start_color();
    use_default_colors();
    curs_set(0);

    init_pair(CORRECT_COL_PAIR, COLOR_GREEN, -1);
    init_pair(CORRECT_CHAR_COL_PAIR, COLOR_YELLOW, -1);
    init_pair(INCORRECT_COL_PAIR, -1, -1);
    init_pair(SELECTED_COL_PAIR, COLOR_RED, -1);
    init_pair(UNSELECTED_COL_PAIR, -1, -1);
    w.keypad(false);

    w
}

fn fini() {
    endwin();
}

fn draw_char(c: char, begy: i32, begx: i32) -> WordleChar {
    let window = newwin(CELL_H as i32, CELL_W as i32, begy, begx);
    window.keypad(true);

    window.mv(CHAR_POS_Y, CHAR_POS_X);
    window.addch(c);

    let mut out = WordleChar {
        c,
        window,
        result: CharResult::Incorrect,
    };
    draw_border(&out);
    set_char_result(&mut out, CharResult::Incorrect, false);
    out.window.refresh();

    out
}

fn result_col(result: CharResult) -> i16 {
    match result {
        CharResult::Correct => CORRECT_COL_PAIR,
        CharResult::CorrectChar => CORRECT_CHAR_COL_PAIR,
        CharResult::Incorrect => INCORRECT_COL_PAIR,
    }
}

fn put_char(c: &WordleChar, is_selected: bool) {
    // there is definitely a better way but this works
    c.window.mv(CHAR_POS_Y, CHAR_POS_X);
    let col = if is_selected {
        SELECTED_COL_PAIR
    } else {
        result_col(c.result)
    };
    c.window.color_set(col);
    c.window.addch(c.c);
    c.window.refresh();
}

fn set_char_result(c: &mut WordleChar, result: CharResult, is_selected: bool) {
    c.result = result;
    c.window.mv(CHAR_POS_Y, CHAR_POS_X);
    put_char(c, is_selected);
    draw_border(c);
    c.window.refresh();
}

fn draw_word(word: &str, begy: i32, mut begx: i32) -> Vec<WordleChar> {
    let mut out = vec![];
    for c in word.chars().map(|c| c.to_ascii_uppercase()) {
        let new = draw_char(c, begy, begx);
        begx = new.window.get_beg_x() + new.window.get_max_x() + 1 + CELL_BUFF;
        out.push(new);
    }
    out
}

fn draw_border(c: &WordleChar) {
    let w = &c.window;
    w.color_set(result_col(c.result));
    w.draw_box('|', '-');
    w.refresh();
}

fn select(c: &mut WordleChar) {
    draw_border(c);
    set_char_result(c, c.result, true);
}

fn deselect(c: &mut WordleChar) {
    draw_border(c);
    set_char_result(c, c.result, false);
}

fn left(word: &mut Vec<WordleChar>, i: &mut usize) {
    if *i > 0 {
        deselect(&mut word[*i]);
        *i -= 1;
        select(&mut word[*i]);
    }
}

fn right(word: &mut Vec<WordleChar>, i: &mut usize) {
    if *i < word.len() - 1 {
        deselect(&mut word[*i]);
        *i += 1;
        select(&mut word[*i]);
    }
}

fn up(c: &mut WordleChar) {
    let new_result = match c.result {
        CharResult::Correct => CharResult::Incorrect,
        CharResult::CorrectChar => CharResult::Correct,
        CharResult::Incorrect => CharResult::CorrectChar,
    };
    set_char_result(c, new_result, true);
}

fn down(c: &mut WordleChar) {
    let new_result = match c.result {
        CharResult::Correct => CharResult::CorrectChar,
        CharResult::CorrectChar => CharResult::Incorrect,
        CharResult::Incorrect => CharResult::Correct,
    };
    set_char_result(c, new_result, true);
}

fn poll_guess_result(guess: &str, begy: &mut i32, begx: i32) -> WordPattern {
    let mut word = draw_word(guess, *begy, begx);
    let first = &mut word[0];
    *begy = first.window.get_beg_y() + first.window.get_max_y() + 1 + BETWEEN_GUESS_BUFF as i32;
    let mut i = 0;
    select(first);
    loop {
        match word[0].window.getch() {
            Some(Input::KeyLeft) => left(&mut word, &mut i),
            Some(Input::KeyRight) => right(&mut word, &mut i),
            Some(Input::KeyUp) => up(&mut word[i]),
            Some(Input::KeyDown) => down(&mut word[i]),
            Some(Input::Character('\n')) | Some(Input::Character('\r')) => break,
            _ => continue,
        }
    }
    deselect(&mut word[i]);

    let mut out = [CharResult::Incorrect; WORD_LEN];
    word.iter()
        .zip(out.iter_mut())
        .for_each(|(c, o)| *o = c.result);
    WordPattern::new(guess, out)
}

fn print_intro(w: &Window) {
    w.printw("
Let's play Wordle! I just need you to mark my guesses.
Use the arrow keys to set the colour of each character in the guess, and press enter to confirm.\n\n");

    w.color_set(CORRECT_COL_PAIR);
    w.printw("correct");
    w.color_set(CORRECT_CHAR_COL_PAIR);
    w.printw("     wrong position");
    w.color_set(INCORRECT_COL_PAIR);
    w.printw("     incorrect");

    w.refresh();
}

fn main() {
    let answers = answers();
    let guesses = guesses();

    let main_window = init();
    print_intro(&main_window);

    let guess = first_guess();
    let (mut begy, begx) = (6, 0);
    let result = play_game(guess, answers.clone(), guesses.clone(), |guess| {
        poll_guess_result(guess, &mut begy, begx)
    });
    println!("{}\r", "\n".repeat(CELL_H));
    match result {
        Err(msg) => println!("{}", msg.0),
        Ok((_, count)) => println!(
            "Solved in {} guess{}!",
            count,
            if count == 1 { "" } else { "es" }
        ),
    }

    println!("\r\nPress any key to exit.");
    std::io::stdout().flush().unwrap();
    std::io::stdin().read_exact(&mut [0]).unwrap();

    fini();
}
