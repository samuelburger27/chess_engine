use crate::board_representation::{board::{Board, Turn, PLAYER_COUNT, WHITE}, piece::{Piece, PIECE_COUNT}, position::Position};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;

pub type ZobristHash = u64;

pub struct ZobristTable {
    // hash for every tile piece type combination
    pub piece_square: [[[ZobristHash; Position::MAX_POS];PIECE_COUNT]; PLAYER_COUNT],
    pub white_to_move: ZobristHash,
    pub castle_rights: [ZobristHash; 4],
    pub en_passant_file: [ZobristHash; 8],
}

impl ZobristTable {

    pub fn new(seed: Option<u64>) -> Self {
        let mut rng = Pcg64Mcg::from_entropy();
        if let Some(seed) = seed {
            rng = Pcg64Mcg::seed_from_u64(seed);
        }
        let mut table = ZobristTable {
            piece_square: [[[0; Position::MAX_POS];PIECE_COUNT]; PLAYER_COUNT],
            //piece_square: [0; Position::MAX_POS * PLAYER_COUNT * PIECE_COUNT],
            white_to_move: rng.gen(),
            castle_rights: [0;4],
            en_passant_file: [0;8],
        };
        for player in 0..PLAYER_COUNT {
            for piece in 0..PIECE_COUNT {
                for pos in 0..Position::MAX_POS {
                    table.piece_square[player][piece][pos] = rng.gen();
                }
            }
        }
        for i in 0..4 {
            table.castle_rights[i] = rng.gen();
        }

        for i in 0..8 {
            table.en_passant_file[i] = rng.gen();
        }
        table
    }

    pub fn hash_position(&self, board: &Board) -> ZobristHash {
        let mut hash: ZobristHash = 0;
        for pos in 0..Position::MAX_POS {
            if let Some((piece, color)) = board.get_piece_at(Position::new(pos)) {
                hash ^= self.piece_square[color as usize][piece as usize][pos];
            }
        }
        if board.turn == WHITE {
            hash ^=self.white_to_move;
        }

        for i in 0..4 {
            if board.castle_rights.castle_at_index(i) {
                hash ^=self.castle_rights[i];
            }
        }

        if board.en_passant.is_not_empty() {
            let (file,_) = Position::new(board.en_passant.trailing_zeros()).get_file_and_rank();
            hash ^=self.en_passant_file[file]
        }

        hash
    }
    
}

