use super::bitboard::Bitboard;

pub const NOT_A_FILE: Bitboard = Bitboard(0xFEFEFEFEFEFEFEFE);
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
pub const B_KING_CASTLE_EMPTY: Bitboard =
    Bitboard(0b01100000_00000000_00000000_00000000_00000000_00000000_00000000_00000000);
pub const B_QUEEN_CASTLE_EMPTY: Bitboard =
    Bitboard(0b00001110_00000000_00000000_00000000_00000000_00000000_00000000_00000000);

pub const W_KING_CASTLE_EMPTY: Bitboard = Bitboard(0b01100000);
pub const W_QUEEN_CASTLE_EMPTY: Bitboard = Bitboard(0b00001110);
