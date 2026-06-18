//! Startup helpers.

use std::sync::LazyLock;

use crate::chess_engine::computed_boards::{BISHOP_ATTACKS, ROOK_ATTACKS, ZOBRIST_TABLE};

/// Eagerly initialises the lazily-built lookup tables (the sliding-piece magic
/// attack tables and the Zobrist table).
///
/// Calling this at startup moves the one-off table-generation cost out of the
/// first move generation or search. It is optional — the tables initialise
/// themselves on first use — but recommended so timing is not skewed.
pub fn init_tables() {
    LazyLock::force(&BISHOP_ATTACKS);
    LazyLock::force(&ROOK_ATTACKS);
    LazyLock::force(&ZOBRIST_TABLE);
}
