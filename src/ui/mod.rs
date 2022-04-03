use std::{
    io::{Stdout, Write},
    process::exit,
};
use wordle::*;

use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{self, Color, ContentStyle, StyledContent, Stylize},
    terminal::{self, disable_raw_mode, enable_raw_mode},
};

pub const CORRECT_COL: Color = Color::Green;
pub const CORRECT_CHAR_COL: Color = Color::Yellow;
pub const UNKNOWN_CHAR_COL: Color = Color::Reset;
pub const INCORRECT_COL: Color = Color::DarkGrey;
pub const SELECTED_COL: Color = Color::Red;

const RESTART_KEY_DESC: &str = "Control-R";
const QUIT_KEY_DESC: &str = "Esc";
const QUIT_CODE: KeyCode = KeyCode::Esc;

pub enum GameControl {
    Restart,
    Quit,
}

pub const SIDE_BUFF: u16 = 2;
pub const BETWEEN_GUESS_BUFF: u16 = 0;
pub const CELL_W: u16 = 3 + SIDE_BUFF;
pub const CELL_H: u16 = 3;
pub const CELL_BUFF: u16 = 0;
pub const CHAR_POS_Y: u16 = 1;
pub const CHAR_POS_X: u16 = 2;

pub struct ScreenPos {
    pub col: u16,
    pub row: u16,
}
pub struct ScreenSize {
    pub cols: u16,
    pub rows: u16,
}

pub struct Window {
    pub pos: ScreenPos,
    pub size: ScreenSize,
}

pub struct WordleChar {
    pub c: char,
    pub window: Window,
    pub result: CharResult,
}

pub fn init(stdout: &mut Stdout) -> crossterm::Result<()> {
    enable_raw_mode()?;
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        terminal::Clear(terminal::ClearType::All),
        cursor::Hide,
        cursor::MoveTo(0, 0)
    )?;
    Ok(())
}

pub fn fini(stdout: &mut Stdout) -> crossterm::Result<()> {
    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
    disable_raw_mode()?;
    exit(0);
}

pub fn print_colour_explanation(stdout: &mut Stdout) -> crossterm::Result<()> {
    execute!(
        stdout,
        style::PrintStyledContent("\r\ncorrect".with(CORRECT_COL)),
        style::PrintStyledContent("    wrong position".with(CORRECT_CHAR_COL)),
        style::PrintStyledContent("    incorrect".with(INCORRECT_COL)),
        style::Print("\n\r"),
    )?;

    Ok(())
}

pub fn print_controls_explanation(stdout: &mut Stdout) -> crossterm::Result<()> {
    execute!(
        stdout,
        style::Print(format!(
            "\rPress {} to quit, {} to restart.\n",
            QUIT_KEY_DESC, RESTART_KEY_DESC
        ))
    )
}

pub fn print_solved_msg(stdout: &mut Stdout, guess_count: u32) -> crossterm::Result<()> {
    execute!(
        stdout,
        style::Print(format!(
            "Solved in {} guess{}!\n",
            guess_count,
            if guess_count == 1 { "" } else { "es" }
        ))
    )
}

pub fn draw_box(
    stdout: &mut Stdout,
    window: &Window,
    style: &ContentStyle,
) -> crossterm::Result<()> {
    let (cols, rows) = terminal::size()?;
    for y in window.pos.row..(window.pos.row + window.size.rows).min(rows) {
        for x in window.pos.col..(window.pos.col + window.size.cols).min(cols) {
            let content = if y == window.pos.row {
                if x == window.pos.col {
                    "┌"
                } else if x < window.pos.col + window.size.cols - 1 {
                    "─"
                } else {
                    "┐"
                }
            } else if y < window.pos.row + window.size.rows - 1 {
                if x == window.pos.col || x == window.pos.col + window.size.cols - 1 {
                    "│"
                } else {
                    continue;
                }
            } else if x == window.pos.col {
                "└"
            } else if x < window.pos.col + window.size.cols - 1 {
                "─"
            } else {
                "┘"
            };

            queue!(
                stdout,
                cursor::MoveTo(x, y),
                style::PrintStyledContent(StyledContent::new(*style, content,))
            )?;
        }
    }
    stdout.flush()?;
    Ok(())
}

pub fn draw_char(
    stdout: &mut Stdout,
    c: char,
    row: u16,
    col: u16,
) -> crossterm::Result<WordleChar> {
    let window = Window {
        pos: ScreenPos { col, row },
        size: ScreenSize {
            cols: CELL_W,
            rows: CELL_H,
        },
    };
    let wordle_char = WordleChar {
        c,
        window,
        result: CharResult::Unknown,
    };

    draw_border(stdout, &wordle_char)?;
    update_char(stdout, &wordle_char, false)?;

    Ok(wordle_char)
}

pub fn result_col(result: CharResult) -> Color {
    match result {
        CharResult::Correct => CORRECT_COL,
        CharResult::CorrectChar => CORRECT_CHAR_COL,
        CharResult::Incorrect => INCORRECT_COL,
        CharResult::Unknown => UNKNOWN_CHAR_COL,
    }
}

pub fn get_styled_char(c: &WordleChar, is_selected: bool) -> StyledContent<char> {
    c.c.to_ascii_uppercase().with(if is_selected {
        SELECTED_COL
    } else {
        result_col(c.result)
    })
}

pub fn update_char(
    stdout: &mut Stdout,
    c: &WordleChar,
    is_selected: bool,
) -> crossterm::Result<()> {
    execute!(
        stdout,
        cursor::MoveTo(c.window.pos.col + CHAR_POS_X, c.window.pos.row + CHAR_POS_Y),
        style::PrintStyledContent(get_styled_char(c, is_selected))
    )?;

    draw_border(stdout, c)?;
    Ok(())
}

pub fn draw_word(
    stdout: &mut Stdout,
    word: &str,
    mut col: u16,
    row: &mut u16,
) -> crossterm::Result<Vec<WordleChar>> {
    let mut out = vec![];
    for c in word.chars() {
        let new = draw_char(stdout, c, *row, col)?;
        col = new.window.pos.col + new.window.size.cols + 1 + CELL_BUFF;
        out.push(new);
    }

    *row += CELL_H + BETWEEN_GUESS_BUFF;
    Ok(out)
}

pub fn draw_border(stdout: &mut Stdout, c: &WordleChar) -> crossterm::Result<()> {
    draw_box(
        stdout,
        &c.window,
        &ContentStyle::new().with(result_col(c.result)),
    )?;
    Ok(())
}

pub fn score_guess(
    stdout: &mut Stdout,
    word: &mut Vec<WordleChar>,
    result: [CharResult; WORD_LEN],
) -> crossterm::Result<()> {
    word.iter_mut().zip(result).for_each(|(c, r)| c.result = r);
    for c in word {
        update_char(stdout, c, false)?;
    }
    Ok(())
}

pub fn is_quit_event(e: KeyEvent) -> bool {
    matches!(
        e,
        KeyEvent {
            code: KeyCode::Char('c' | 'd' | 'z'),
            modifiers: KeyModifiers::CONTROL,
        } | KeyEvent {
            code: QUIT_CODE,
            ..
        }
    )
}

pub fn is_restart_event(e: KeyEvent) -> bool {
    matches!(
        e,
        KeyEvent {
            code: KeyCode::Char('r'),
            modifiers: KeyModifiers::CONTROL,
        }
    )
}
