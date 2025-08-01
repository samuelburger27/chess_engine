use super::bitboard::Bitboard;

/// Bit representation of file A.
pub const FILE_A: Bitboard =
    Bitboard(0b00000001_00000001_00000001_00000001_00000001_00000001_00000001_00000001);
/// Bit representation of file B.
pub const FILE_B: Bitboard =
    Bitboard(0b00000010_00000010_00000010_00000010_00000010_00000010_00000010_00000010);
/// Bit representation of file C.
pub const FILE_C: Bitboard =
    Bitboard(0b00000100_00000100_00000100_00000100_00000100_00000100_00000100_00000100);
/// Bit representation of file D.
pub const FILE_D: Bitboard =
    Bitboard(0b00001000_00001000_00001000_00001000_00001000_00001000_00001000_00001000);
/// Bit representation of file E.
pub const FILE_E: Bitboard =
    Bitboard(0b00010000_00010000_00010000_00010000_00010000_00010000_00010000_00010000);
/// Bit representation of file F.
pub const FILE_F: Bitboard =
    Bitboard(0b00100000_00100000_00100000_00100000_00100000_00100000_00100000_00100000);
/// Bit representation of file G.
pub const FILE_G: Bitboard =
    Bitboard(0b01000000_01000000_01000000_01000000_01000000_01000000_01000000_01000000);
/// Bit representation of file H.
pub const FILE_H: Bitboard =
    Bitboard(0b10000000_10000000_10000000_10000000_10000000_10000000_10000000_10000000);

pub const NOT_A_FILE: Bitboard = Bitboard(0xFEFEFEFEFEFEFEFE);
pub const NOT_H_FILE: Bitboard = Bitboard(0x7F7F7F7F7F7F7F7F);

/// Bit representation of rank 1.
pub const RANK_1: Bitboard = Bitboard(0x0000_0000_0000_00FF);
/// Bit representation of rank 2.
pub const RANK_2: Bitboard = Bitboard(0x0000_0000_0000_FF00);
/// Bit representation of rank 3.
pub const RANK_3: Bitboard = Bitboard(0x0000_0000_00FF_0000);
/// Bit representation of rank 4.
pub const RANK_4: Bitboard = Bitboard(0x0000_0000_FF00_0000);
/// Bit representation of rank 5.
pub const RANK_5: Bitboard = Bitboard(0x0000_00FF_0000_0000);
/// Bit representation of rank 6.
pub const RANK_6: Bitboard = Bitboard(0x0000_FF00_0000_0000);
/// Bit representation of rank 7.
pub const RANK_7: Bitboard = Bitboard(0x00FF_0000_0000_0000);
/// Bit representation of rank 8.
pub const RANK_8: Bitboard = Bitboard(0xFF00_0000_0000_0000);

/// Bits for starting occupancy boards for a white pawn.
pub const START_W_PAWN: Bitboard =
    Bitboard(0b00000000_00000000_00000000_00000000_00000000_00000000_11111111_00000000);
/// Bits for starting occupancy boards for a white knight.
pub const START_W_KNIGHT: Bitboard =
    Bitboard(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_01000010);
/// Array for starting occupancy boards for a white bishop.
pub const START_W_BISHOP: Bitboard =
    Bitboard(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00100100);
/// Bits for starting occupancy boards for a white rook.
pub const START_W_ROOK: Bitboard =
    Bitboard(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_10000001);
/// Bits for starting occupancy boards for a white queen.
pub const START_W_QUEEN: Bitboard =
    Bitboard(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00001000);
/// Bits for starting occupancy boards for a white king.
pub const START_W_KING: Bitboard =
    Bitboard(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00010000);

/// Bits for starting occupancy boards for a black pawn.
pub const START_B_PAWN: Bitboard =
    Bitboard(0b00000000_11111111_00000000_00000000_00000000_00000000_00000000_00000000);
/// Bits for starting occupancy boards for a black knight.
pub const START_B_KNIGHT: Bitboard =
    Bitboard(0b01000010_00000000_00000000_00000000_00000000_00000000_00000000_00000000);
/// Bits for starting occupancy boards for a black bishop.
pub const START_B_BISHOP: Bitboard =
    Bitboard(0b00100100_00000000_00000000_00000000_00000000_00000000_00000000_00000000);
/// Bits for starting occupancy boards for a black rook.
pub const START_B_ROOK: Bitboard =
    Bitboard(0b10000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000);
/// Bits for starting occupancy boards for a black queen.
pub const START_B_QUEEN: Bitboard =
    Bitboard(0b00001000_00000000_00000000_00000000_00000000_00000000_00000000_00000000);
/// Bits for starting occupancy boards for a black king.
pub const START_B_KING: Bitboard =
    Bitboard(0b00010000_00000000_00000000_00000000_00000000_00000000_00000000_00000000);
// bits that should be empty when castling
pub const B_KING_CASTLE_EMPTY: Bitboard =
    Bitboard(0b01100000_00000000_00000000_00000000_00000000_00000000_00000000_00000000);
pub const B_QUEEN_CASTLE_EMPTY: Bitboard =
    Bitboard(0b00001110_00000000_00000000_00000000_00000000_00000000_00000000_00000000);

pub const W_KING_CASTLE_EMPTY: Bitboard = Bitboard(0b01100000);
pub const W_QUEEN_CASTLE_EMPTY: Bitboard = Bitboard(0b00001110);
