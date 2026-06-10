# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                        # build
cargo test                         # run all tests (perft tests)
cargo test run_perft_tests         # run the single perft test suite
cargo run                          # start the engine (expects UCI input)
```

The engine speaks UCI on stdin/stdout. Supported commands: `uci`, `isready`, `ucinewgame`, `position startpos|fen ... [moves ...]`, `go` (with `depth`, `movetime`, `wtime/btime/winc/binc`, `infinite`, or `perft N`), `stop`, `d` (print board), `quit`. Search runs on a background thread so `stop` works mid-search; bare `go` defaults to a 3-second budget.

## Architecture

The crate is a UCI chess engine written in Rust. The public surface in `src/lib.rs` is three modules: `chess_engine` (all core logic), `perft` (node-count correctness testing), and `uci` (protocol).

### Board representation (`src/chess_engine/`)

`Board` (`board.rs`) is the central struct. It holds:
- `piece_boards: [Bitboard; 12]` — one 64-bit board per (piece, colour) pair. Index formula: `piece as usize + (turn as usize * PIECE_COUNT)`, so white pieces occupy indices 0–5 and black 6–11.
- `player_boards: [Bitboard; 2]` and `empty_tiles` — derived aggregates, recomputed after every move via `compute_bitboards()`.
- `turn: Turn` — `bool` where `WHITE = false` and `BLACK = true`; opponent is always `!turn`.
- `en_passant`, `castle_rights`, `halfmove_count`, `fullmove_count`.
- `zobrist_key: ZobristHash` — maintained incrementally; every `add_piece`/`remove_piece` call XORs the relevant table entry.
- `history: Vec<StateDelta>` — stack used by `unmake_move`.

`Bitboard` (`bitboard.rs`) wraps `u64` and implements all bitwise operators plus helpers like `reset_lsb`, `trailing_zeros`, `is_not_empty`.

`Move` (`move.rs`) packs everything into 16 bits:
- bits 0–5: destination square
- bits 6–11: origin square
- bits 12–13: promotion piece (knight/bishop/rook/queen)
- bits 14–15: special move type (normal/en-passant/castle/promotion)

`Position` (`position.rs`) wraps a square index (0–63) and provides `try_rank_file_offset`, `algebraic_notation`, and conversion helpers.

### Lookup tables (`computed_boards.rs`)

`KNIGHT_MOVES` and `KING_RING_MOVES` are `const` arrays computed at compile time.

`BISHOP_ATTACKS` and `ROOK_ATTACKS` are `LazyLock<Vec<Bitboard>>` populated via magic-bitboard indexing. **`init_tables()` must be called at startup** (`main.rs` does this) to force their initialisation. The magic numbers themselves live as `const` arrays (`BISHOP_MAGICS`, `ROOK_MAGICS`); `magic_tables.rs` can regenerate them with `find_magics`.

### Move generation (`move_generation.rs`)

`Board::generate_moves` = pseudo-legal generation → filter by `would_check` (make/unmake the move and test `in_check`).

`generate_pseudo_non_castle_moves` is deliberately separate from castle generation to break the recursion: checking whether a king passes through an attacked square calls attack generation, which must not recurse into castle logic.

### Make/Unmake (`make_move.rs`)

`commit_verified_move` pushes a `StateDelta` (carrying the full pre-move Zobrist hash, used for repetition detection) before mutating anything, then handles all four `SpecialMove` variants while maintaining the Zobrist key incrementally. `unmake_move` pops `history`, reverses the move, and restores the Zobrist key wholesale from the delta. Unit tests at the bottom of the file assert the incremental hash always matches a full recompute.

### Engine (`src/chess_engine/engine/`)

`search.rs` — iterative deepening over fail-soft negamax with alpha-beta pruning, quiescence search (captures/promotions only), MVV-LVA move ordering, and draw detection (fifty-move, twofold repetition via the history hashes, insufficient material). Mate scores are encoded as `MATE_SCORE - ply`. `search_position` polls an `AtomicBool` stop flag and an optional deadline every 2048 nodes and prints UCI `info` lines per completed depth; the UCI layer owns time allocation (`clock/25 + inc/2`, capped at half the clock).

`evaluation.rs` — tapered material + piece-square-table evaluation. Game phase is computed from the remaining piece count; king PST is interpolated between middlegame and endgame tables. PSTs are written rank-8-first, so white squares are flipped (`sq ^ 56`) via `pst_index` and black squares index directly.

### Testing

`tests/perft_tests.rs` contains perft cases from the Chess Programming Wiki (initial position depth 5, Kiwipete depth 4, and several others). These are the correctness oracle for move generation. The `compare.sh` / `run_stockfish.sh` scripts and `debug_logger.py` are used to diff perft output against Stockfish.
