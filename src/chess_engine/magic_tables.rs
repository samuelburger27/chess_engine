//! [Magic bitboards][magic]: O(1) attack lookup for sliding pieces.
//!
//! A rook or bishop's reachable squares depend only on the occupied squares
//! along its rays (the "blockers"). A [`MagicEntry`] turns that blocker pattern
//! into a small hash index — `(blockers & mask) * magic >> shift` — that points
//! into a precomputed table of attack bitboards. The magic multipliers are
//! found offline by [`find_magics`] and baked into
//! `computed_boards.rs`; at runtime [`magic_index`](MagicEntry::magic_index) is
//! all that is needed.
//!
//! [magic]: https://analog-hors.github.io/site/magic-bitboards/

use super::bitboard::Bitboard;
use crate::chess_engine::{
    computed_boards::{BISHOP_BLOCKERS, ROOK_BLOCKERS},
    constants::{BISHOP_DELTAS, EMPTY_BIT_B, ROOK_DELTAS},
    move_generation::get_sliding_moves,
    position::Position,
};
use rand::{RngExt, SeedableRng};
use rand_pcg::Pcg64Mcg;

// inspired by https://analog-hors.github.io/site/magic-bitboards/

/// The per-square magic-hashing parameters for one sliding piece.
#[derive(Clone, Copy)]
pub struct MagicEntry {
    /// The relevant-blocker mask (the ray squares, excluding the board edge).
    pub mask: Bitboard,
    /// The magic multiplier that spreads masked blockers into a unique index.
    pub magic: u64,
    /// Right-shift applied after multiplying, sizing the index to the table.
    pub shift: u8,
    /// Base offset of this square's block within the shared attack table.
    pub offset: usize,
}
impl MagicEntry {
    /// Hashes a blocker configuration to its index *within this square's block*
    /// (callers add [`offset`](Self::offset) to reach the shared table).
    ///
    /// ```
    /// use sabertooth::chess_engine::magic_tables::MagicEntry;
    /// use sabertooth::chess_engine::bitboard::Bitboard;
    ///
    /// let entry = MagicEntry { mask: Bitboard::full(), magic: 0xdead_beef, shift: 58, offset: 0 };
    /// // no blockers hashes to 0 regardless of the magic
    /// assert_eq!(entry.magic_index(Bitboard::new()), 0);
    /// ```
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn magic_index(&self, blockers: Bitboard) -> usize {
        let blockers = blockers & self.mask;
        let hash = blockers.0.wrapping_mul(self.magic);
        (hash >> self.shift) as usize
    }
}

/// Searches random candidates until it finds a magic number that maps every
/// blocker configuration for `pos` to a collision-free table slot, returning
/// the resulting [`MagicEntry`] and its filled table.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn find_magic(
    deltas: &[(i8, i8); 4],
    blockers: &[Bitboard],
    pos: Position,
    index_bits: u8,
    rng: &mut Pcg64Mcg,
) -> (MagicEntry, Vec<Bitboard>) {
    let mask = blockers[pos.as_usize()];
    let shift = 64 - index_bits;
    loop {
        // Magics require a low number of active bits, so we AND
        // by two more random values to cut down on the bits set.
        let magic = rng.random::<u64>() & rng.random::<u64>() & rng.random::<u64>();
        let magic_entry = MagicEntry {
            mask,
            magic,
            shift,
            offset: 0,
        };
        if let Ok(table) = try_make_table(deltas, pos, &magic_entry) {
            return (magic_entry, table);
        }
    }
}

/// Attempts to fill the attack table for `pos` using `magic_entry`, iterating
/// every blocker subset. Returns `Err` if two different attack sets collide on
/// one slot, meaning the candidate magic is unusable.
#[allow(clippy::trivially_copy_pass_by_ref, clippy::cast_possible_truncation)]
fn try_make_table(
    deltas: &[(i8, i8); 4],
    pos: Position,
    magic_entry: &MagicEntry,
) -> Result<Vec<Bitboard>, String> {
    let index_bits = Position::MAX_POS as u8 - magic_entry.shift;
    let mut table = vec![EMPTY_BIT_B; 1 << index_bits];
    // Iterate all configurations of blockers
    let mut blockers = EMPTY_BIT_B;
    loop {
        let moves = get_sliding_moves(deltas, pos, blockers);
        let table_entry = &mut table[magic_entry.magic_index(blockers)];
        if table_entry.is_empty() {
            // Write to empty slot
            *table_entry = moves;
        } else if *table_entry != moves {
            // Having two different move sets in the same slot is a hash collision
            return Err("Hash collision".to_string());
        }

        // Carry-Rippler trick that enumerates all subsets of the mask, getting us all blockers.
        // https://www.chessprogramming.org/Traversing_Subsets_of_a_Set#All_Subsets_of_any_Set
        blockers.0 = blockers.0.wrapping_sub(magic_entry.mask.0) & magic_entry.mask.0;
        if blockers.is_empty() {
            // Finished enumerating all blocker configurations
            break;
        }
    }
    Ok(table)
}

/// Finds magics for all 64 squares of one slider and prints them as the Rust
/// source (a `MagicEntry` array plus its table size) that gets pasted into
/// `computed_boards.rs`.
#[allow(clippy::trivially_copy_pass_by_ref, clippy::cast_possible_truncation)]
fn find_and_print_all_magics(
    deltas: &[(i8, i8); 4],
    blockers: &[Bitboard],
    slider_name: &str,
    rng: &mut Pcg64Mcg,
) {
    println!("#[rustfmt::skip]");
    println!("pub const {slider_name}_MAGICS: &[MagicEntry; MAX_POS] = &[");
    let mut total_table_size = 0;
    for i in 0..64 {
        let pos = Position::new(i);
        let index_bits = blockers[pos.as_usize()].count_bits() as u8;
        let (entry, table) = find_magic(deltas, blockers, pos, index_bits, rng);
        // offset is added to denote the start of each segment.
        println!(
            "    MagicEntry {{ mask: Bitboard(0x{:016X}), magic: 0x{:016X}, shift: {}, offset: {} }},",
            entry.mask.0, entry.magic, entry.shift, total_table_size
        );
        total_table_size += table.len();
    }
    println!("];");
    println!("pub const {slider_name}_TABLE_SIZE: usize = {total_table_size};");
}

/// Offline tool: regenerates and prints all rook and bishop magic tables.
///
/// Not used during play — the generated numbers already live as `const` arrays
/// in `computed_boards.rs`. Pass `Some(seed)` to reproduce a known set.
pub fn find_magics(magic_num_seed: Option<u64>) {
    let mut rng = magic_num_seed.map_or_else(
        || Pcg64Mcg::from_rng(&mut rand::rng()),
        Pcg64Mcg::seed_from_u64,
    );

    find_and_print_all_magics(&BISHOP_DELTAS, &BISHOP_BLOCKERS, "BISHOP", &mut rng);
    find_and_print_all_magics(&ROOK_DELTAS, &ROOK_BLOCKERS, "ROOK", &mut rng);
}
