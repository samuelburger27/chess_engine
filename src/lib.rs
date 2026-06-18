//! A UCI-compatible chess engine.
//!
//! The crate is organised into three modules:
//!
//! - [`chess_engine`] — all the core logic: the [`Board`](chess_engine::board::Board)
//!   position type, move generation, make/unmake, and the search/evaluation
//!   [`engine`](chess_engine::engine).
//! - [`perft`] — node-count testing, the correctness oracle for move generation.
//! - [`uci`] — the [Universal Chess Interface] protocol loop that drives the
//!   engine over stdin/stdout.
//!
//! # Example
//!
//! ```
//! use chess_engine::chess_engine::board::{Board, WHITE};
//! use chess_engine::chess_engine::engine::search::find_best_move;
//! use chess_engine::chess_engine::utils::init_tables;
//!
//! // Build the lookup tables once at startup.
//! init_tables();
//!
//! // From the opening position, White has twenty legal moves.
//! let mut board = Board::new_start_pos().unwrap();
//! assert_eq!(board.generate_moves(WHITE).len(), 20);
//!
//! // Ask the search for a move (a shallow depth keeps the example fast).
//! let result = find_best_move(&board, 3);
//! assert!(result.best_move.is_some());
//! ```
//!
//! [Universal Chess Interface]: https://www.chessprogramming.org/UCI
#![warn(missing_docs)]

pub mod chess_engine;
pub mod perft;
pub mod uci;
