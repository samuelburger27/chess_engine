//! A [`Move`] packs an origin square, destination square, promotion choice, and
//! special-move flag into a single 16-bit value.
//!
//! # Bit layout
//!
//! | Bits  | Meaning |
//! |-------|---------|
//! | 0–5   | destination square (`0..64`) |
//! | 6–11  | origin square (`0..64`) |
//! | 12–13 | promotion piece ([`PROMOTE_TO_KNIGHT`] … [`PROMOTE_TO_QUEEN`]) |
//! | 14–15 | special-move type ([`NORMAL_MOVE`], [`EN_PASSANT`], [`CASTLING`], [`PROMOTION`]) |
//!
//! The promotion bits are only meaningful when the special-move type is
//! [`PROMOTION`]; for every other move they default to "queen" and are ignored.
//! Constructors do **not** bounds-check squares — they assume callers pass
//! valid [`Position`]s (which are themselves `0..64`).
//!
//! # Examples
//!
//! ```
//! use chess_engine::chess_engine::r#move::Move;
//! use chess_engine::chess_engine::position::Position;
//!
//! let m = Move::new_default(Position::new(12), Position::new(28)); // e2 -> e4
//! assert_eq!(m.get_origin(), Position::new(12));
//! assert_eq!(m.get_dest(), Position::new(28));
//! assert_eq!(m.to_string(), "e2e4");
//! ```

use std::fmt::{self, Debug};

use super::position::Position;
use crate::chess_engine::{
    board::{Turn, WHITE},
    piece::Piece,
    r#const::{
        B_KING_CASTLE_DEST, B_KING_START, B_QUEEN_CASTLE_DEST, W_KING_CASTLE_DEST, W_KING_START,
        W_QUEEN_CASTLE_DEST,
    },
};

/// The kind of a move, decoded from a [`Move`]'s special-move bits.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum SpecialMove {
    /// A pawn reaching the last rank; the promotion bits select the new piece.
    Promotion,
    /// A pawn capturing en passant.
    EnPassant,
    /// A king/rook castling move (encoded as the king's two-square step).
    Castle,
    /// Any ordinary move or capture.
    NormalMove,
}

/// A chess move encoded in 16 bits. See the [module documentation](self) for
/// the bit layout.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move(u16);

/// Promotion-bit value selecting a knight (set in bits 12–13).
pub const PROMOTE_TO_KNIGHT: u16 = 0b01 << 12;
/// Promotion-bit value selecting a bishop (set in bits 12–13).
pub const PROMOTE_TO_BISHOP: u16 = 0b10 << 12;
/// Promotion-bit value selecting a rook (set in bits 12–13).
pub const PROMOTE_TO_ROOK: u16 = 0b11 << 12;
/// Promotion-bit value selecting a queen (set in bits 12–13); also the default.
pub const PROMOTE_TO_QUEEN: u16 = 0b00 << 12;

/// Special-move flag for an ordinary move or capture (bits 14–15).
pub const NORMAL_MOVE: u16 = 0b00 << 14;
/// Special-move flag for an en-passant capture (bits 14–15).
pub const EN_PASSANT: u16 = 0b01 << 14;
/// Special-move flag for castling (bits 14–15).
pub const CASTLING: u16 = 0b10 << 14;
/// Special-move flag for a promotion (bits 14–15).
pub const PROMOTION: u16 = 0b11 << 14;

// default move (promote to queen, no special move)
const DEFAULT_MOVE: u16 = PROMOTE_TO_QUEEN | NORMAL_MOVE;

// TODO refactor

impl Move {
    /// Wraps a raw 16-bit encoding without validation.
    #[must_use] 
    pub const fn make_raw(data: u16) -> Self {
        Self(data)
    }

    /// Returns the raw 16-bit encoding.
    #[must_use] 
    pub const fn get_raw(&self) -> u16 {
        self.0
    }

    /// Returns the destination square (bits 0–5).
    #[must_use] 
    pub const fn get_dest(&self) -> Position {
        let mask = 0b0000000000111111u16;
        Position::new((mask & self.0) as usize)
    }

    /// Returns the origin square (bits 6–11).
    #[must_use] 
    pub const fn get_origin(&self) -> Position {
        let mask = 0b0000_1111_1100_0000u16;
        Position::new(((mask & self.0) >> 6) as usize)
    }

    /// Returns the `(origin, destination)` pair.
    #[must_use] 
    pub const fn get_org_and_dest(&self) -> (Position, Position) {
        (self.get_origin(), self.get_dest())
    }

    /// Decodes the special-move type (bits 14–15).
    ///
    /// ```
    /// use chess_engine::chess_engine::r#move::{Move, SpecialMove};
    /// use chess_engine::chess_engine::position::Position;
    /// let m = Move::new_default(Position::new(12), Position::new(28));
    /// assert_eq!(m.get_special_move(), SpecialMove::NormalMove);
    /// ```
    #[must_use] 
    pub const fn get_special_move(&self) -> SpecialMove {
        let mask = 0b1100000000000000u16;
        match self.0 & mask {
            PROMOTION => SpecialMove::Promotion,
            EN_PASSANT => SpecialMove::EnPassant,
            CASTLING => SpecialMove::Castle,
            _ => SpecialMove::NormalMove,
        }
    }

    /// Decodes the promotion piece (bits 12–13). Only meaningful when
    /// [`get_special_move`](Self::get_special_move) is [`SpecialMove::Promotion`];
    /// otherwise returns [`Piece::Queen`] (the default bit pattern).
    #[must_use] 
    pub const fn get_promotion(&self) -> Piece {
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

    /// Builds an ordinary move (no special flag) from `origin` to `destination`.
    #[must_use] 
    pub fn new_default(origin: Position, destination: Position) -> Self {
        let mask = Self::create_move_mask(origin, destination);
        Self(mask | DEFAULT_MOVE)
    }

    /// Builds the four promotion moves (knight, bishop, rook, queen) for a pawn
    /// advancing from `origin` to `destination`.
    ///
    /// ```
    /// use chess_engine::chess_engine::r#move::Move;
    /// use chess_engine::chess_engine::position::Position;
    /// // e7 (52) -> e8 (60)
    /// let promos = Move::new_promote(Position::new(52), Position::new(60));
    /// let strings: Vec<String> = promos.iter().map(|m| m.to_string()).collect();
    /// assert_eq!(strings, ["e7e8n", "e7e8b", "e7e8r", "e7e8q"]);
    /// ```
    #[must_use] 
    pub fn new_promote(origin: Position, destination: Position) -> [Self; 4] {
        let mask: u16 = Self::create_move_mask(origin, destination) | PROMOTION;
        [
            Self(mask | PROMOTE_TO_KNIGHT),
            Self(mask | PROMOTE_TO_BISHOP),
            Self(mask | PROMOTE_TO_ROOK),
            Self(mask | PROMOTE_TO_QUEEN),
        ]
    }

    /// Builds a move from `origin` to `destination` carrying the given `special`
    /// flag (one of [`NORMAL_MOVE`], [`EN_PASSANT`], [`CASTLING`], [`PROMOTION`],
    /// optionally OR-ed with a promotion-piece value).
    #[must_use] 
    pub fn new_special(origin: Position, destination: Position, special: u16) -> Self {
        let mask = Self::create_move_mask(origin, destination) | special;
        Self(mask)
    }

    /// Builds the castling move for `turn` on the given side, encoded as the
    /// king's two-square step (e.g. white king-side is `e1`→`g1`).
    #[must_use] 
    pub fn new_castle(king_side: bool, turn: Turn) -> Self {
        if turn == WHITE {
            if king_side {
                return Self::new_special(W_KING_START, W_KING_CASTLE_DEST, CASTLING);
            }
            return Self::new_special(W_KING_START, W_QUEEN_CASTLE_DEST, CASTLING);
        } else if king_side {
            return Self::new_special(B_KING_START, B_KING_CASTLE_DEST, CASTLING);
        }
        Self::new_special(B_KING_START, B_QUEEN_CASTLE_DEST, CASTLING)
    }
}

/// Formats the move in long algebraic / UCI notation: origin and destination
/// squares, with the promotion piece letter appended for promotions (e.g.
/// `"e2e4"`, `"e7e8q"`).
impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            self.get_origin().algebraic_notation(),
            self.get_dest().algebraic_notation()
        )?;
        if self.get_special_move() == SpecialMove::Promotion {
            write!(f, "{}", self.get_promotion().to_notation())?;
        }
        Ok(())
    }
}

/// Formats every decoded field (raw bits, origin, destination, special-move
/// type, promotion piece) for debugging.
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
