//! Fixed [`Bitboard`] masks used during move generation: file/rank masks and
//! the squares that must be empty for castling.

use super::bitboard::Bitboard;

/// Every square except the a-file; AND with this before an eastward shift to
/// stop pawns/pieces wrapping around the board edge.
pub const NOT_A_FILE: Bitboard = Bitboard(0xFEFE_FEFE_FEFE_FEFE);
/// Every square except the h-file; AND with this before a westward shift.
pub const NOT_H_FILE: Bitboard = Bitboard(0x7F7F_7F7F_7F7F_7F7F);

/// Bit representation of rank 1.
pub const RANK_1: Bitboard = Bitboard(0x0000_0000_0000_00FF);
/// Bit representation of rank 2.
pub const RANK_2: Bitboard = Bitboard(0x0000_0000_0000_FF00);
/// Bit representation of rank 7.
pub const RANK_7: Bitboard = Bitboard(0x00FF_0000_0000_0000);
/// Bit representation of rank 8.
pub const RANK_8: Bitboard = Bitboard(0xFF00_0000_0000_0000);

/// Bit representation of the black squares
pub const BLACK_SQUARES: Bitboard = Bitboard(0xAA55_AA55_AA55_AA55);

/// Bit representation of the a-file.
pub const FILE_A: Bitboard = Bitboard(0x0101_0101_0101_0101);

/// One mask per file, indexed `a = 0` through `h = 7`.
pub const FILE_MASKS: [Bitboard; 8] = generate_file_masks();

/// One mask per rank, indexed rank 1 = `0` through rank 8 = `7`.
pub const RANK_MASKS: [Bitboard; 8] = generate_rank_masks();

/// For each file, the mask of its neighbouring file(s) — the own file is
/// excluded. A pawn with no friendly pawn in its entry here is *isolated*.
pub const ADJACENT_FILE_MASKS: [Bitboard; 8] = generate_adjacent_file_masks();

const fn generate_file_masks() -> [Bitboard; 8] {
    let mut masks = [Bitboard(0); 8];
    let mut file = 0;
    while file < 8 {
        masks[file] = Bitboard(FILE_A.0 << file);
        file += 1;
    }
    masks
}

const fn generate_rank_masks() -> [Bitboard; 8] {
    let mut masks = [Bitboard(0); 8];
    let mut rank = 0;
    while rank < 8 {
        masks[rank] = Bitboard(RANK_1.0 << (8 * rank));
        rank += 1;
    }
    masks
}

const fn generate_adjacent_file_masks() -> [Bitboard; 8] {
    let mut masks = [Bitboard(0); 8];
    let mut file = 0;
    while file < 8 {
        if file > 0 {
            masks[file].0 |= FILE_A.0 << (file - 1);
        }
        if file < 7 {
            masks[file].0 |= FILE_A.0 << (file + 1);
        }
        file += 1;
    }
    masks
}

// bits that should be empty when castling
/// Squares (f8, g8) that must be empty for Black to castle king-side.
pub const B_KING_CASTLE_EMPTY: Bitboard =
    Bitboard(0b0110_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000);
/// Squares (b8, c8, d8) that must be empty for Black to castle queen-side.
pub const B_QUEEN_CASTLE_EMPTY: Bitboard =
    Bitboard(0b0000_1110_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000);

/// Squares (f1, g1) that must be empty for White to castle king-side.
pub const W_KING_CASTLE_EMPTY: Bitboard = Bitboard(0b0110_0000);
/// Squares (b1, c1, d1) that must be empty for White to castle queen-side.
pub const W_QUEEN_CASTLE_EMPTY: Bitboard = Bitboard(0b0000_1110);
