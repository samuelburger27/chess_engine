use crate::board_representation::{bitboard::Bitboard, position::Position};

pub const EMPTY_BIT_B: Bitboard = Bitboard::new();

pub const MAX_POS: usize = 64;

pub const W_QUEEN_START: Position = Position::from_file_and_rank(3, 0);
pub const B_QUEEN_START: Position = Position::from_file_and_rank(3, 7);

pub const W_KING_START: Position = Position::from_file_and_rank(4, 0);
pub const B_KING_START: Position = Position::from_file_and_rank(4, 7);

pub const W_KING_ROOK_START: Position = Position::from_file_and_rank(7, 0);
pub const B_KING_ROOK_START: Position = Position::from_file_and_rank(7, 7);

pub const W_QUEEN_ROOK_START: Position = Position::from_file_and_rank(0, 0);
pub const B_QUEEN_ROOK_START: Position = Position::from_file_and_rank(0, 7);

pub const W_KING_CASTLE_DEST: Position = Position::from_file_and_rank(6, 0);
pub const B_KING_CASTLE_DEST: Position = Position::from_file_and_rank(6, 7);

pub const W_QUEEN_CASTLE_DEST: Position = Position::from_file_and_rank(2, 0);
pub const B_QUEEN_CASTLE_DEST: Position = Position::from_file_and_rank(2, 7);

pub const W_KING_SIDE_BISHOP_START: Position = Position::from_file_and_rank(5, 0);
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

pub const ROOK_DELTAS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
pub const BISHOP_DELTAS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
