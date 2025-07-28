use crate::board_representation::{board::{Turn, WHITE}, r#const::{B_KING_CASTLE_DEST, B_KING_START, B_QUEEN_CASTLE_DEST, W_KING_CASTLE_DEST, W_KING_START, W_QUEEN_CASTLE_DEST}, piece::Piece};
use super::position::Position;

#[derive(PartialEq)]
pub enum SpecialMove{
    Promotion,
    EnPassant,
    Castle,
    NormalMove,
}


// 16-bit unsigned integer to represent a move
// bit 0-5: to square (0-63)
// bit 6-11: from square (0-63)
// bit 12-13: promotion piece type (knight, bishop, rook, queen)
// bit 14-15: special move (promotion, en passant, castling)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move(u16);

// promotions
pub const PROMOTE_TO_KNIGHT: u16 = 0b01 << 12;
pub const PROMOTE_TO_BISHOP: u16 = 0b10 << 12;
pub const PROMOTE_TO_ROOK: u16 = 0b11 << 12;
pub const PROMOTE_TO_QUEEN: u16 = 0b00 << 12;

// special moves
pub const PROMOTION: u16 = 0b00;
pub const EN_PASSANT: u16 = 0b01 << 14;
pub const CASTLING: u16 = 0b10 << 14;
pub const NORMAL_MOVE: u16 = 0b11 << 14;

// default move (promote to queen, no special move)
const DEFAULT_MOVE: u16 = PROMOTE_TO_QUEEN | NORMAL_MOVE;

impl Move {

    pub fn get_dest(&self) -> Position {
        let mask = 0b0000000000111111u16;
        return Position::new((mask & self.0) as usize);
    }

    pub fn get_origin(&self) -> Position {
        let mask = 0b0000111111000000u16;
        return Position::new((mask & self.0) as usize);
    }

    pub fn get_org_and_dest(&self) -> (Position, Position) {
        (self.get_origin(), self.get_dest())
    }

    pub fn get_special_move(&self) -> SpecialMove {
        let mask = 0b1100000000000000u16;
        match self.0 & mask {
            PROMOTION => SpecialMove::Promotion,
            EN_PASSANT => SpecialMove::EnPassant,
            CASTLING => SpecialMove::Castle,
            _ => SpecialMove::NormalMove,
        }
    }

    pub fn get_promotion(&self) -> Piece {
        let mask: u16 = 0b0001100000000000u16;
        match self.0 & mask {
            PROMOTE_TO_KNIGHT => Piece::Knight,
            PROMOTE_TO_BISHOP => Piece::Bishop,
            PROMOTE_TO_QUEEN => Piece::Queen,
            PROMOTE_TO_ROOK => Piece::Rook,
            _ => Piece::None
        }
    }

    fn create_move_mask(origin: Position, destination: Position) -> u16 {
        let from_square = usize::from(origin) as u16;
        let to_square = usize::from(destination) as u16;
        to_square | (from_square & 0b00111111 << 6)
    }
    // default move, no special move
    pub fn new_default(origin: Position, destination: Position) -> Self {
        let mask = Self::create_move_mask(origin, destination);
        Move(mask | DEFAULT_MOVE)
    }

    // return all possible promotions
    pub fn new_promote(origin: Position, destination: Position) -> [Self; 4] {
        let mask: u16 = Move::create_move_mask(origin, destination) | PROMOTION;
        [
            Move(mask | PROMOTE_TO_KNIGHT),
            Move(mask | PROMOTE_TO_BISHOP),
            Move(mask | PROMOTE_TO_ROOK),
            Move(mask | PROMOTE_TO_QUEEN),
        ]
    }

    pub fn new_special(origin: Position, destination: Position, special: u16) -> Self {
        let mask = Self::create_move_mask(origin, destination) | special;
        Move(mask)
    }

    pub fn new_castle(king_side: bool, turn: Turn) -> Self {
        if turn == WHITE {
            if king_side {
                return Move::new_special(W_KING_START, W_KING_CASTLE_DEST, CASTLING);
            }
            else {
                return Move::new_special(W_KING_START, W_QUEEN_CASTLE_DEST, CASTLING);
            }
        }
        else if king_side {
            return Move::new_special(B_KING_START, B_KING_CASTLE_DEST, CASTLING);    
        }
        return Move::new_special(B_KING_START, B_QUEEN_CASTLE_DEST, CASTLING);    


    }
}

impl ToString for Move {
    fn to_string(&self) -> String {
        let mut result = self.get_origin().algebraic_notation() + &self.get_dest().algebraic_notation();
        if self.get_special_move() == SpecialMove::Promotion {
            result += &self.get_promotion().to_notation();
        }
        return result;
    }
}