# Bugs

 - Game score can be incremented multiple times while exploring history.
   - Separate out "end of round" from "start of next round"?
   - Clone the `Round` and decouple exploration from game state.
 - History widget needs to be scrolly.

# Functionality

 - Exploration mode:
   - Only offer at the end of round during normal gameplay.
   - Launch directly into exploration mode from a save file (no game).
 - Collapsible branches in history widget.
   - Sigil? "(+N)" suffix?

# Development

 - Long-term: Game abstraction layer
   - Common traits for core state machine & log functionality?
   - Wait until we have a second game. Hearts? Spades? Sheepshead?
