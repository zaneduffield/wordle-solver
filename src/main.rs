use clap::{Parser, Subcommand};

mod play;
mod solve;
mod ui;

#[derive(Parser)]
#[clap(name = "wordle")]
#[clap(about = "a wordle solving / playing client", long_about = None)]
#[clap(author)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a wordle game
    Play {
        /// (optional) the solution to the wordle game
        ///
        /// When specified, you will be playing a game with this as the answer
        answer: Option<String>,
    },
    /// Start a wordle solver
    Solve {
        /// (optional) the wordle to be solved
        ///
        /// When specified, the solver will try to crack it in a self-scored game
        answer: Option<String>,
    },
}

fn main() -> crossterm::Result<()> {
    let args = Cli::parse();
    match &args.command {
        Commands::Play { answer } => play::play(answer.as_ref())?,
        Commands::Solve { answer: None } => solve::solve()?,
        Commands::Solve {
            answer: Some(answer),
        } => solve::solve_for_answer(answer)?,
    }

    Ok(())
}
