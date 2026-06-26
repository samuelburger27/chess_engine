//! The playing engine: how the program chooses a move.
//!
//! [`search`] explores the game tree (iterative-deepening alpha-beta with
//! quiescence) and [`evaluation`] scores the quiet leaf positions it reaches
//! (tapered material plus piece-square tables). Together they turn a
//! [`Board`](super::board::Board) into a best move and its evaluation.

pub mod evaluation;
pub mod search;
pub mod transposition;
