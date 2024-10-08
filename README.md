# Deckard

This project aspires to be a card game engine with:

- An interactive terminal UI
- Robot player implementations
- A serializable tree-structured log format
- A browser for exploring "what if" scenarios

## Project status

This project is very much a work-in-progress.
At this stage, it's just a playground for exercise & learning.

The idea is to provide reusable components for a card game engine.
For now, the only card game implemented is euchre.

Most of the code here is prototype quality.
I've never written a game engine.
I've never designed or implemented a complex terminal UI.
I'm getting pretty comfortable with Rust, but I'm no expert on ergonomics.

## Quickstart

To run the game, simply:

```console
$ cargo run
```

## Demo

Basic gameplay:

![Demo](images/demo.gif?raw=true)

During gameplay, you can open the history explorer with the `!` key.
You can seek to a point in history, and experiment with alternative choices.
If you want to override the robot implementation, you can use `@` to toggle robot autoplay.

![History demo](images/history.gif?raw=true)

## Key Bindings

- `←↑→↓` / `hjkl`: Cursor navigation. 
- `↵` / `␣`: Select highlighted item.
- `s`: Saves the current round to `euchre.json`.
- `!`: Opens the history browser.
- `@`: Toggles robot autoplay.
- `?`: What would the robot do?
- `q`: Quits the game (or closes the history browser).

## To Do

### Chores

- Improve test coverage.

### Functionality

- Exploration mode features
  - Only offer at the end of round during normal gameplay.
  - Optionally display "hidden" state (other players' hands).
  - Edit other player's actions.
- Standalone exploration mode (from save file)
- Collapsible branches in history widget
  - Sigil? "(+N)" suffix?

### Future

- Game abstraction layer
  - Common traits for core state machine & log functionality?
  - Wait until we add some more games. Hearts? Spades? Sheepshead?
- Robot implementation bakeoffs
- Play analysis & coaching
- Full game logs
- HTTP/JS frontend
