//! Core chess logic: board representation, move generation, make/unmake, and
//! the search/evaluation engine.
//!
//! The central type is [`board::Board`], a bitboard-based position that knows
//! how to [generate legal moves](board::Board::generate_moves), apply and undo
//! them, and detect checks and draws. Supporting modules cover the value types
//! the board is built from ([`bitboard::Bitboard`], [`position::Position`],
//! [`piece::Piece`], [`castle_rights::CastleRights`], the `r#move::Move` type), the
//! pre-computed attack tables ([`magic_tables`]), and the [`engine`] that picks
//! the best move.
//!
//! Move generation correctness is pinned down by the perft tests in
//! `tests/perft_tests.rs`; see [`crate::perft`].
pub mod bitboard;
pub mod board;
pub mod castle_rights;
mod computed_boards;
mod r#const;
pub mod engine;
mod fen_parser;
mod game_state;
pub mod magic_tables;
mod make_move;
mod masks;
pub mod r#move;
mod move_generation;
pub mod piece;
pub mod position;
pub mod utils;
mod zobrist;
