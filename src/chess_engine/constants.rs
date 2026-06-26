//! Shared board constants: the empty bitboard, the fixed starting/destination
//! squares involved in castling, and the directional offsets used to walk rays
//! during move generation.
//!
//! Square offsets are expressed in `Bitboard`/`Position` index units, where
//! moving one rank north is `+8` and one file east is `+1`.

use crate::chess_engine::{bitboard::Bitboard, position::Position};

/// An empty bitboard, handy as a `const` starting value.
pub const EMPTY_BIT_B: Bitboard = Bitboard::new();

/// White queen's starting square (`d1`).
pub const W_QUEEN_START: Position = Position::from_file_and_rank(3, 0);
/// Black queen's starting square (`d8`).
pub const B_QUEEN_START: Position = Position::from_file_and_rank(3, 7);

/// White king's starting square (`e1`).
pub const W_KING_START: Position = Position::from_file_and_rank(4, 0);
/// Black king's starting square (`e8`).
pub const B_KING_START: Position = Position::from_file_and_rank(4, 7);

/// White king-side rook's starting square (`h1`).
pub const W_KING_ROOK_START: Position = Position::from_file_and_rank(7, 0);
/// Black king-side rook's starting square (`h8`).
pub const B_KING_ROOK_START: Position = Position::from_file_and_rank(7, 7);

/// White queen-side rook's starting square (`a1`).
pub const W_QUEEN_ROOK_START: Position = Position::from_file_and_rank(0, 0);
/// Black queen-side rook's starting square (`a8`).
pub const B_QUEEN_ROOK_START: Position = Position::from_file_and_rank(0, 7);

/// White king's square after king-side castling (`g1`).
pub const W_KING_CASTLE_DEST: Position = Position::from_file_and_rank(6, 0);
/// Black king's square after king-side castling (`g8`).
pub const B_KING_CASTLE_DEST: Position = Position::from_file_and_rank(6, 7);

/// White king's square after queen-side castling (`c1`).
pub const W_QUEEN_CASTLE_DEST: Position = Position::from_file_and_rank(2, 0);
/// Black king's square after queen-side castling (`c8`).
pub const B_QUEEN_CASTLE_DEST: Position = Position::from_file_and_rank(2, 7);

/// White king-side bishop's starting square (`f1`); the king passes over it
/// when castling king-side.
pub const W_KING_SIDE_BISHOP_START: Position = Position::from_file_and_rank(5, 0);
/// Black king-side bishop's starting square (`f8`).
pub const B_KING_SIDE_BISHOP_START: Position = Position::from_file_and_rank(5, 7);

/// Direction of going north on a chessboard.
pub const NORTH: i8 = 8;
/// Direction of going south on a chessboard.
pub const SOUTH: i8 = -8;
/// Direction of going west on a chessboard.
#[allow(unused)]
pub const WEST: i8 = -1;
/// Direction of going east on a chessboard.
#[allow(unused)]
pub const EAST: i8 = 1;
/// Direction of going northeast on a chessboard.
pub const NORTH_EAST: i8 = 9;
/// Direction of going northwest on a chessboard.
pub const NORTH_WEST: i8 = 7;
/// Direction of going southeast on a chessboard.
pub const SOUTH_EAST: i8 = -7;
/// Direction of going southwest on a chessboard.
pub const SOUTH_WEST: i8 = -9;

/// The four `(d_file, d_rank)` steps a rook moves along (orthogonals).
pub const ROOK_DELTAS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
/// The four `(d_file, d_rank)` steps a bishop moves along (diagonals).
pub const BISHOP_DELTAS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
