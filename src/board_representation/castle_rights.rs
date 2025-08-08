use std::fmt::Debug;
use crate::board_representation::board::Turn;

#[derive(Clone, Copy, PartialEq)]
pub struct CastleRights {
    // white_king_side, white_queen_side, black_king_side, black_queen_side
    flags: u8,
}

const WHITE_KING_SIDE: u8 = 0b0001;
const WHITE_QUEEN_SIDE: u8 = 0b0010;
const BLACK_KING_SIDE: u8 = 0b0100;
const BLACK_QUEEN_SIDE: u8 = 0b1000;

impl CastleRights {
    pub fn make_default() -> Self {
        CastleRights { flags: 0b1111 }
    }

    pub fn make(
        white_king_side: bool,
        white_queen_side: bool,
        black_king_side: bool,
        black_queen_side: bool,
    ) -> Self {
        let mut flags = 0;
        if white_king_side {
            flags |= WHITE_KING_SIDE;
        }
        if white_queen_side {
            flags |= WHITE_QUEEN_SIDE;
        }
        if black_king_side {
            flags |= BLACK_KING_SIDE;
        }
        if black_queen_side {
            flags |= BLACK_QUEEN_SIDE;
        }
        CastleRights { flags }
    }

    pub fn can_castle(&self, turn: Turn, king_side: bool) -> bool {
        let index = 2 * usize::from(turn) + usize::from(!king_side);
        self.flags & (1 << index) != 0
    }

    pub fn remove_castle_right(&mut self, turn: Turn, king_side: bool) {
        let index = 2 * usize::from(turn) + usize::from(!king_side);
        self.flags &= !(1 << index);
    }

    pub fn castle_at_index(&self, index: usize) -> bool {
        self.flags & (1 << index) != 0
    }
}

impl Debug for CastleRights {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let wk = self.flags & WHITE_KING_SIDE != 0;
        let wq = self.flags & WHITE_QUEEN_SIDE != 0;
        let bk = self.flags & BLACK_KING_SIDE != 0;
        let bq = self.flags & BLACK_QUEEN_SIDE != 0;

        write!(
            f,
            "CastleRights {{ white_king_side: {}, white_queen_side: {}, black_king_side: {}, black_queen_side: {} }}",
            wk, wq, bk, bq
        )
    }
}