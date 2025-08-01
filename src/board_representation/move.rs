use std::fmt::Debug;

use super::position::Position;
use crate::board_representation::{
    board::{Turn, WHITE},
    piece::Piece,
    r#const::{
        B_KING_CASTLE_DEST, B_KING_START, B_QUEEN_CASTLE_DEST, W_KING_CASTLE_DEST, W_KING_START,
        W_QUEEN_CASTLE_DEST,
    },
};

#[derive(PartialEq, Debug, Clone)]
pub enum SpecialMove {
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move(u16);

// promotions
pub const PROMOTE_TO_KNIGHT: u16 = 0b01 << 12;
pub const PROMOTE_TO_BISHOP: u16 = 0b10 << 12;
pub const PROMOTE_TO_ROOK: u16 = 0b11 << 12;
pub const PROMOTE_TO_QUEEN: u16 = 0b00 << 12;

// special moves
pub const NORMAL_MOVE: u16 = 0b00 << 14;
pub const EN_PASSANT: u16 = 0b01 << 14;
pub const CASTLING: u16 = 0b10 << 14;
pub const PROMOTION: u16 = 0b11 << 14;

// default move (promote to queen, no special move)
const DEFAULT_MOVE: u16 = PROMOTE_TO_QUEEN | NORMAL_MOVE;

// TODO refactor

impl Move {
    pub fn make_raw(data: u16) -> Move {
        Move(data)
    }

    pub fn get_raw(&self) -> u16 {
        self.0
    }

    pub fn get_dest(&self) -> Position {
        let mask = 0b0000000000111111u16;
        return Position::new((mask & self.0) as usize);
    }

    pub fn get_origin(&self) -> Position {
        let mask = 0b0000_1111_1100_0000u16;
        return Position::new(((mask & self.0) >> 6) as usize);
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
        let mask: u16 = 0b0011000000000000u16;
        match self.0 & mask {
            PROMOTE_TO_KNIGHT => Piece::Knight,
            PROMOTE_TO_BISHOP => Piece::Bishop,
            PROMOTE_TO_ROOK => Piece::Rook,
            _ => Piece::Queen,
        }
    }

    fn create_move_mask(origin: Position, destination: Position) -> u16 {
        let origin_square = usize::from(origin) as u16;
        let dest_square = usize::from(destination) as u16;
        dest_square | ((origin_square & 0b00111111) << 6)
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
            } else {
                return Move::new_special(W_KING_START, W_QUEEN_CASTLE_DEST, CASTLING);
            }
        } else if king_side {
            return Move::new_special(B_KING_START, B_KING_CASTLE_DEST, CASTLING);
        }
        return Move::new_special(B_KING_START, B_QUEEN_CASTLE_DEST, CASTLING);
    }
}

impl ToString for Move {
    fn to_string(&self) -> String {
        let mut result =
            self.get_origin().algebraic_notation() + &self.get_dest().algebraic_notation();
        if self.get_special_move() == SpecialMove::Promotion {
            result += &self.get_promotion().to_notation();
        }
        return result;
    }
}

impl Debug for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let origin = self.get_origin();
        let dest = self.get_dest();
        let special = self.get_special_move();
        let promo = self.get_promotion();

        write!(
            f,
            "Move {{\n  raw: {},\n  origin: {} ({:?}),\n  dest: {} ({:?}),\n  special: {:?},\n  promotion: {:?}\n}}",
            self.0,
            origin.as_usize(),
            origin.algebraic_notation(),
            dest.as_usize(),
            dest.algebraic_notation(),
            special,
            promo
        )
    }
}
