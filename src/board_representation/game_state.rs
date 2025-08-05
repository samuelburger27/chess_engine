use crate::board_representation::{
    bitboard::Bitboard, castle_rights::CastleRights, piece::Piece, r#move::Move,
};

// used for undoing moves
#[derive(Clone, Debug, PartialEq)]
pub struct StateDelta {
    pub move_: Move,
    pub captured_piece: Option<Piece>,
    pub en_pass: Bitboard,
    pub castle_rights: CastleRights,
    pub halfmove: u8,
    // TODO zobrist hash
}

impl StateDelta {
    pub fn new(
        move_: Move,
        captured_piece: Option<Piece>,
        en_pass: Bitboard,
        castle_rights: CastleRights,
        halfmove: u8,
    ) -> StateDelta {
        StateDelta {
            move_,
            captured_piece,
            en_pass,
            castle_rights,
            halfmove,
        }
    }
}
