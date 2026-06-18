//! [Zobrist hashing][zob]: a 64-bit fingerprint of a position used for
//! repetition detection (and a natural key for transposition tables).
//!
//! Each independent feature of a position — a piece on a square, the side to
//! move, each castling right, the en-passant file — is assigned a random
//! 64-bit number in a [`ZobristTable`]. A position's hash is the XOR of the
//! numbers for the features it has. Because XOR is its own inverse, a feature
//! can be toggled in or out in O(1), which is what lets [`Board`] keep its hash
//! up to date incrementally as moves are made and unmade.
//!
//! [zob]: https://www.chessprogramming.org/Zobrist_Hashing

use crate::chess_engine::{
    board::{Board, PLAYER_COUNT, WHITE},
    piece::PIECE_COUNT,
    position::Position,
};
use rand::{RngExt, SeedableRng};
use rand_pcg::Pcg64Mcg;

/// A Zobrist position hash.
pub type ZobristHash = u64;

/// The table of random numbers that defines a Zobrist hash function. All boards
/// in a game must use the same table for their hashes to be comparable.
pub struct ZobristTable {
    // hash for every tile piece type combination
    /// Random key per `[colour][piece][square]`.
    pub piece_square: [[[ZobristHash; Position::MAX_POS]; PIECE_COUNT]; PLAYER_COUNT],
    /// Key XOR-ed in when it is White's turn.
    pub white_to_move: ZobristHash,
    /// Key per castling right (indexed as in [`CastleRights`](super::castle_rights)).
    pub castle_rights: [ZobristHash; 4],
    /// Key per en-passant file (`a`–`h`).
    pub en_passant_file: [ZobristHash; 8],
}

impl ZobristTable {
    /// Builds a table of random keys. Pass `Some(seed)` for a deterministic
    /// table (used so every board in the program shares one fixed function);
    /// `None` seeds from the OS RNG.
    pub fn new(seed: Option<u64>) -> Self {
        let mut rng = seed.map_or_else(
            || Pcg64Mcg::from_rng(&mut rand::rng()),
            Pcg64Mcg::seed_from_u64,
        );
        let mut table = Self {
            piece_square: [[[0; Position::MAX_POS]; PIECE_COUNT]; PLAYER_COUNT],
            //piece_square: [0; Position::MAX_POS * PLAYER_COUNT * PIECE_COUNT],
            white_to_move: rng.random(),
            castle_rights: [0; 4],
            en_passant_file: [0; 8],
        };
        for player in 0..PLAYER_COUNT {
            for piece in 0..PIECE_COUNT {
                for pos in 0..Position::MAX_POS {
                    table.piece_square[player][piece][pos] = rng.random();
                }
            }
        }
        for i in 0..4 {
            table.castle_rights[i] = rng.random();
        }

        for i in 0..8 {
            table.en_passant_file[i] = rng.random();
        }
        table
    }

    /// Computes a position's hash from scratch by XOR-ing the key for every
    /// feature present (pieces, side to move, castling rights, en-passant file).
    /// [`Board`] uses this to seed its hash and the tests use it to check the
    /// incremental updates never drift.
    pub fn hash_position(&self, board: &Board) -> ZobristHash {
        let mut hash: ZobristHash = 0;
        for pos in 0..Position::MAX_POS {
            if let Some((piece, color)) = board.get_piece_at(Position::new(pos)) {
                hash ^= self.piece_square[usize::from(color)][piece as usize][pos];
            }
        }
        if board.turn == WHITE {
            hash ^= self.white_to_move;
        }

        for i in 0..4 {
            if board.castle_rights.castle_at_index(i) {
                hash ^= self.castle_rights[i];
            }
        }

        if board.en_passant.is_not_empty() {
            let (file, _) = Position::new(board.en_passant.trailing_zeros()).get_file_and_rank();
            hash ^= self.en_passant_file[file];
        }

        hash
    }
}

#[cfg(test)]
mod tests {
    use super::ZobristTable;
    use crate::chess_engine::board::Board;

    #[test]
    fn same_seed_gives_same_hashes() {
        let a = ZobristTable::new(Some(42));
        let b = ZobristTable::new(Some(42));
        let board = Board::new_start_pos().unwrap();
        // a fixed seed produces a reproducible hashing function
        assert_eq!(a.hash_position(&board), b.hash_position(&board));
    }

    #[test]
    fn different_positions_hash_differently() {
        let table = ZobristTable::new(Some(42));
        let start = Board::new_start_pos().unwrap();
        // same pieces, but Black to move -> different hash
        let black_to_move =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
        assert_ne!(
            table.hash_position(&start),
            table.hash_position(&black_to_move)
        );
    }
}
