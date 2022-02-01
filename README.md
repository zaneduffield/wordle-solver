# World Solver / Player
A terminal UI for solving and playing Wordle.

### Practice solving random Wordle puzzles with `wordle play`

https://user-images.githubusercontent.com/60605769/151975044-dd46838d-d575-42b6-8c2e-5b4318f84269.mp4

### See how the computer plays with `wordle solve`

https://user-images.githubusercontent.com/60605769/151975059-ab7bb299-9f7e-47ae-8e01-549f8b9b8a1a.mp4

If you can't be bothered scoring the computer's guesses (and trust that it won't cheat) you can give it the answer as the next command-line argument and let it score itself.

## Build
Install [rustup](https://www.rust-lang.org/) if you haven't already.
```sh
# release build because the `solve` functionality is too slow in debug mode
cargo build --release
```

## Run
```
./wordle { solve | play } [word]
```
