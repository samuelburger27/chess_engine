# SaberTooth

SaberTooth is a UCI-compatible chess engine written in Rust.

## Features

- **Bitboard representation** — 64-bit integers per (piece, colour) pair with magic-bitboard sliding piece attacks
- **UCI protocol** — plug into any UCI-compatible GUI (Arena, Cutechess, etc.)
- **Iterative deepening** — fail-soft negamax with alpha-beta pruning and aspiration windows
- **Search pruning** — principal variation search, null-move pruning, late move reductions, and check extensions
- **Transposition table** — Zobrist-keyed cache shared across search threads
- **Lazy SMP** — parallel search over multiple threads (configurable via the `Threads` UCI option)
- **Quiescence search** — captures and promotions (all evasions while in check), with delta and SEE pruning
- **Move ordering** — TT move, MVV-LVA (Most Valuable Victim, Least Valuable Attacker), killer moves, and the history heuristic
- **Tapered evaluation** — PeSTO material + piece-square tables interpolated between middlegame and endgame phases
- **Positional terms** — passed/isolated/doubled pawns, bishop pair, rook file bonuses, king pawn shield, per-piece mobility, and attack-unit king danger
- **Draw detection** — fifty-move rule, twofold repetition (via Zobrist hashing), insufficient material
- **Time management** — `clock/25 + inc/2` budget, capped at half the clock
- **Perft testing** — correctness oracle for move generation

## Building & Running

```bash
cargo build --release
cargo run --release        # starts the engine, reads UCI from stdin
```

## API Documentation

The crate is documented with rustdoc (module overviews, every public item, and
runnable examples). Build and open it with:

```bash
cargo doc --no-deps --open
```

The doctested examples double as tests; run them with `cargo test --doc`.

## UCI Commands

| Command | Description |
|---|---|
| `uci` | Identify the engine |
| `isready` | Sync with GUI |
| `ucinewgame` | Reset for a new game |
| `position startpos [moves ...]` | Set position from start |
| `position fen <fen> [moves ...]` | Set position from FEN |
| `go depth <n>` | Search to fixed depth |
| `go movetime <ms>` | Search for a fixed time |
| `go wtime <ms> btime <ms> [winc <ms> binc <ms>]` | Search with clock |
| `go infinite` | Search until `stop` |
| `go perft <n>` | Count nodes at depth n |
| `setoption name Threads value <n>` | Set the number of search threads (1–256) |
| `stop` | Stop a running search |
| `d` | Print the current board |
| `quit` | Exit |

## Testing

```bash
cargo test                     # run all perft tests
cargo test run_perft_tests     # run just the perft suite
```

Perft cases are drawn from the [Chess Programming Wiki](https://www.chessprogramming.org/Perft_Results) (starting position depth 5, Kiwipete depth 4, and several others).

## Architecture

```
src/
├── main.rs                    # entry point, initialises lookup tables, starts UCI loop
├── lib.rs                     # crate root re-exporting chess_engine, perft, uci
├── uci.rs                     # UCI protocol parser and time-allocation logic
├── perft.rs                   # perft node-count runner
└── chess_engine/
    ├── board.rs               # Board struct (bitboards, make/unmake, Zobrist key)
    ├── bitboard.rs            # Bitboard newtype wrapping u64
    ├── moves.rs               # Move packed into 16 bits
    ├── move_generation.rs     # Pseudo-legal generation + legality filter
    ├── make_move.rs           # commit_verified_move / unmake_move; null moves
    ├── game_state.rs          # StateDelta, the per-move undo record
    ├── computed_boards.rs     # Compile-time knight/king tables; magic rook/bishop tables
    ├── magic_tables.rs        # Magic-number generation utility
    ├── masks.rs               # File/rank and castling masks
    ├── constants.rs           # Shared board constants and ray offsets
    ├── zobrist.rs             # Zobrist hashing
    ├── fen_parser.rs          # FEN string parser
    ├── position.rs            # Square index helpers
    ├── piece.rs               # Piece and colour enums
    ├── castle_rights.rs       # Castling rights bitfield
    ├── utils.rs               # Startup helpers (init_tables)
    └── engine/
        ├── search.rs          # Iterative deepening, negamax, quiescence, draw detection
        ├── evaluation.rs      # Tapered material + PST + positional evaluation
        └── transposition.rs   # Zobrist-keyed transposition table
```

### Board Representation

`Board` stores one `u64` bitboard per (piece, colour) pair — 12 boards total. The index formula is `piece + (colour * 6)`, so white occupies indices 0–5 and black 6–11. Sliding piece attacks use magic bitboards initialised at startup via `init_tables()`.

### Move Encoding

Each `Move` is 16 bits: destination (0–5), origin (6–11), promotion piece (12–13), special type — normal / en-passant / castle / promotion (14–15).

### Search

Iterative deepening over fail-soft negamax with aspiration windows, principal variation search, null-move pruning, late move reductions, and check extensions. Leaf positions run a quiescence search over captures and promotions (all evasions while in check) with delta and SEE pruning. Moves are ordered by transposition-table move, MVV-LVA captures, killer moves, and the history heuristic. Parallelism uses Lazy SMP: helper threads search the same root and share results through the transposition table. Each worker polls a shared `AtomicBool` stop flag and an optional deadline every 2048 nodes; a UCI `info` line is printed after each completed depth. Mate scores are encoded as `MATE_SCORE - ply`.
