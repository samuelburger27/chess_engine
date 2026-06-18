//! Fixed [`Bitboard`] masks used during move generation: file/rank masks and
//! the squares that must be empty for castling.

use super::bitboard::Bitboard;

/// Every square except the a-file; AND with this before an eastward shift to
/// stop pawns/pieces wrapping around the board edge.
pub const NOT_A_FILE: Bitboard = Bitboard(0xFEFEFEFEFEFEFEFE);
/// Every square except the h-file; AND with this before a westward shift.
pub const NOT_H_FILE: Bitboard = Bitboard(0x7F7F7F7F7F7F7F7F);

/// Bit representation of rank 1.
pub const RANK_1: Bitboard = Bitboard(0x0000_0000_0000_00FF);
/// Bit representation of rank 2.
pub const RANK_2: Bitboard = Bitboard(0x0000_0000_0000_FF00);
/// Bit representation of rank 7.
pub const RANK_7: Bitboard = Bitboard(0x00FF_0000_0000_0000);
/// Bit representation of rank 8.
pub const RANK_8: Bitboard = Bitboard(0xFF00_0000_0000_0000);

// bits that should be empty when castling
/// Squares (f8, g8) that must be empty for Black to castle king-side.
pub const B_KING_CASTLE_EMPTY: Bitboard =
    Bitboard(0b01100000_00000000_00000000_00000000_00000000_00000000_00000000_00000000);
/// Squares (b8, c8, d8) that must be empty for Black to castle queen-side.
pub const B_QUEEN_CASTLE_EMPTY: Bitboard =
    Bitboard(0b00001110_00000000_00000000_00000000_00000000_00000000_00000000_00000000);

/// Squares (f1, g1) that must be empty for White to castle king-side.
pub const W_KING_CASTLE_EMPTY: Bitboard = Bitboard(0b01100000);
/// Squares (b1, c1, d1) that must be empty for White to castle queen-side.
pub const W_QUEEN_CASTLE_EMPTY: Bitboard = Bitboard(0b00001110);
