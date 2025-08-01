use std::sync::LazyLock;

use crate::board_representation::computed_boards::{BISHOP_ATTACKS, ROOK_ATTACKS};


pub fn init_tables() {
    LazyLock::force(&BISHOP_ATTACKS);
    LazyLock::force(&ROOK_ATTACKS);
}
