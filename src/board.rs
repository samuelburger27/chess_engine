#[derive(Clone, Copy)]
struct Position {
    x: usize,
    y: usize,
}

impl Position {
    fn get_bit_index(&self) -> usize {
        return 8 * self.y + self.x;
    }

    // fn get_algebraic_notation(&self) -> str {

    // }
    // TODO
}

enum Piece {
    Pawn,
    Rook,
    Knight,
    Bishop,
    King,
    Queen,
    None,
}


struct Board {
    white_bitboards: [u64; 6],
    black_bitboards: [u64; 6],
    white_turn: bool,
    // if any pawn can be captured by en passant(just made double move) it will be recorded in this variable
    en_passant: Option<u32>,
    // [white_king_side, white_queen_side, black_king_side, black_queen_side]
    possible_castle: [bool; 4],
}

impl Board {
    
    fn is_position_in_bitboard(pos: Position, bitboard: u64) -> bool {
        let mask = 1u64 << pos.get_bit_index();
        return (bitboard & mask) > 0;
    }

    fn match_index_to_piece(index: usize) -> Result<Piece, ()> {
        match index {
            0 => Ok(Piece::Pawn),
            1 => Ok(Piece::Rook),
            2 => Ok(Piece::Knight),
            3 => Ok(Piece::Bishop),
            4 => Ok(Piece::King),
            5 => Ok(Piece::Queen),
            _ => Err(())
        }
    }

    fn can_castle(&self, king_side: bool) -> bool {
        let index = 2 * usize::from(self.white_turn) + usize::from(!king_side);
        return self.possible_castle[index];
    }


    fn get_piece_and_colour(&self, pos: Position) -> (Piece, bool) {
        for (index, bitboard) in self.white_bitboards.iter().enumerate() {
            if Board::is_position_in_bitboard(pos, *bitboard) {
                return (Board::match_index_to_piece(index).unwrap(), true)
            }
        }
        for (index, bitboard) in self.black_bitboards.iter().enumerate() {
            if Board::is_position_in_bitboard(pos, *bitboard) {
                return (Board::match_index_to_piece(index).unwrap(), false)
            }
        }
        return (Piece::None, true)
    }
}
