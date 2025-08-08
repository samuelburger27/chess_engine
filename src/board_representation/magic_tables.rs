use super::bitboard::Bitboard;
use crate::board_representation::{
    computed_boards::{BISHOP_BLOCKERS, ROOK_BLOCKERS},
    move_generation::get_sliding_moves,
    position::Position,
    r#const::{BISHOP_DELTAS, EMPTY_BIT_B, MAX_POS, ROOK_DELTAS},
};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;

// inspired by https://analog-hors.github.io/site/magic-bitboards/

#[derive(Clone, Copy)]
pub struct MagicEntry {
    pub mask: Bitboard,
    pub magic: u64,
    pub shift: u8,
    pub offset: usize,
}
impl MagicEntry {
    pub fn magic_index(&self, blockers: Bitboard) -> usize {
        let blockers = blockers & self.mask;
        let hash = blockers.0.wrapping_mul(self.magic);
        let index = (hash >> self.shift) as usize;
        index
    }
}


// Given a sliding piece and a square, finds a magic number that
// perfectly maps input blockers into its solution in a hash table
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
        let magic = rng.gen::<u64>() & rng.gen::<u64>() & rng.gen::<u64>();
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

fn try_make_table(
    deltas: &[(i8, i8); 4],
    pos: Position,
    magic_entry: &MagicEntry,
) -> Result<Vec<Bitboard>, String> {
    let index_bits = MAX_POS as u8 - magic_entry.shift;
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

fn find_and_print_all_magics(
    deltas: &[(i8, i8); 4],
    blockers: &[Bitboard],
    slider_name: &str,
    rng: &mut Pcg64Mcg,
) {
    println!("#[rustfmt::skip]");
    println!(
        "pub const {}_MAGICS: &[MagicEntry; MAX_POS] = &[",
        slider_name
    );
    let mut attack_table: Vec<Bitboard> = Vec::new();
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
        attack_table.extend(table);
    }
    println!("];");
    println!(
        "pub const {}_TABLE_SIZE: usize = {};",
        slider_name, total_table_size
    );
}

pub fn find_magics(magic_num_seed: Option<u64>) {
    let mut rng = Pcg64Mcg::from_entropy();
    if let Some(seed) = magic_num_seed {
        rng = Pcg64Mcg::seed_from_u64(seed);
    }

    find_and_print_all_magics(&BISHOP_DELTAS, &BISHOP_BLOCKERS, "BISHOP", &mut rng);
    find_and_print_all_magics(&ROOK_DELTAS, &ROOK_BLOCKERS, "ROOK", &mut rng);
}
