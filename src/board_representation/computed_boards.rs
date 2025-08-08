use std::sync::LazyLock;

use crate::board_representation::{
    r#const::{BISHOP_DELTAS, EMPTY_BIT_B, MAX_POS, ROOK_DELTAS}, magic_tables::MagicEntry, move_generation::get_sliding_moves, position::Position, zobrist::ZobristTable
};

use super::bitboard::Bitboard;

pub const KNIGHT_MOVES: [Bitboard; MAX_POS] = generate_knight_moves();
pub const KING_RING_MOVES: [Bitboard; MAX_POS] = generate_king_ring_moves();

pub const ROOK_BLOCKERS: [Bitboard; MAX_POS] = generate_slide_piece_blockers(&ROOK_DELTAS);
pub const BISHOP_BLOCKERS: [Bitboard; MAX_POS] = generate_slide_piece_blockers(&BISHOP_DELTAS);

pub static BISHOP_ATTACKS: LazyLock<Vec<Bitboard>> = LazyLock::new(|| {
    generate_slide_piece_attack_tables(&BISHOP_DELTAS, BISHOP_MAGICS, BISHOP_TABLE_SIZE)
});
pub static ROOK_ATTACKS: LazyLock<Vec<Bitboard>> = LazyLock::new(|| {
    generate_slide_piece_attack_tables(&ROOK_DELTAS, ROOK_MAGICS, ROOK_TABLE_SIZE)
});
pub static ZOBRIST_TABLE: LazyLock<ZobristTable> = LazyLock::new(|| {
    ZobristTable::new(Some(1234))
});

fn generate_slide_piece_attack_tables(
    slider_deltas: &[(i8, i8); 4],
    magics: &[MagicEntry; MAX_POS],
    table_size: usize,
) -> Vec<Bitboard> {
    let mut table = vec![EMPTY_BIT_B; table_size];
    //table.reserve(table_size);
    for pos in 0..MAX_POS {
        let magic_entry = &magics[pos];
        let mask = magic_entry.mask;

        let mut blockers = EMPTY_BIT_B;
        loop {
            let moves = get_sliding_moves(slider_deltas, Position::new(pos), blockers);
            table[magic_entry.magic_index(blockers) + magic_entry.offset] = moves;

            // Carry-Rippler trick that enumerates all subsets of the mask, getting us all blockers.
            // https://www.chessprogramming.org/Traversing_Subsets_of_a_Set#All_Subsets_of_any_Set
            blockers.0 = blockers.0.wrapping_sub(mask.0) & mask.0;
            if blockers.is_empty() {
                break;
            }
        }
    }
    table
}

const fn generate_slide_piece_blockers(deltas: &[(i8, i8); 4]) -> [Bitboard; MAX_POS] {
    let mut moves = [EMPTY_BIT_B; MAX_POS];
    let mut square = 0;
    while square < MAX_POS {
        let pos = Position::new(square);
        let mut delt_i = 0;
        while delt_i < 4 {
            let (d_file, d_rank) = deltas[delt_i];
            let mut ray = pos;
            while let Some(shifted) = ray.try_rank_file_offset(d_file, d_rank) {
                moves[square].0 |= ray.bitboard().0;
                ray = shifted;
            }
            delt_i += 1
        }
        moves[square].0 &= !pos.bitboard().0;
        square += 1
    }
    moves
}

const fn generate_king_ring_moves() -> [Bitboard; MAX_POS] {
    let mut moves = [Bitboard(0); MAX_POS];
    let mut square = 0;
    while square < MAX_POS {
        let file = square % 8;
        let rank = square / 8;

        // King moves: one square in any direction
        let mut df = -1;
        while df <= 1 {
            let mut dr = -1;
            while dr <= 1 {
                if df == 0 && dr == 0 {
                    dr += 1;
                    continue; // skip the current square
                }
                let target_file = file as isize + df;
                let target_rank = rank as isize + dr;

                if target_file >= 0 && target_file < 8 && target_rank >= 0 && target_rank < 8 {
                    moves[square].set_square((target_rank * 8 + target_file) as usize);
                }
                dr += 1;
            }
            df += 1;
        }
        square += 1;
    }
    moves
}

const fn generate_knight_moves() -> [Bitboard; MAX_POS] {
    let mut moves = [Bitboard(0); MAX_POS];
    let mut square = 0;
    let deltas = [
        (2, 1),
        (1, 2),
        (-1, 2),
        (-2, 1),
        (-2, -1),
        (-1, -2),
        (1, -2),
        (2, -1),
    ];
    while square < MAX_POS {
        // Knight moves: L-shape (2+1 or 1+2)
        // offsets for knight moves
        let mut i = 0;
        while i < 8 {
            let (d_f, d_r) = deltas[i];
            let target_file = (square % 8) as isize + d_f;
            let target_rank = (square / 8) as isize + d_r;
            if target_file >= 0 && target_file < 8 && target_rank >= 0 && target_rank < 8 {
                moves[square].set_square(
                    Position::from_file_and_rank(target_file as usize, target_rank as usize)
                        .as_usize(),
                );
            }
            i += 1;
        }
        square += 1;
    }
    moves
}

// pasted from find_all_magics
#[rustfmt::skip]
pub const BISHOP_MAGICS: &[MagicEntry; MAX_POS] = &[
    MagicEntry { mask: Bitboard(0x0040201008040200), magic: 0x200204104C860080, shift: 58, offset: 0 },
    MagicEntry { mask: Bitboard(0x0000402010080400), magic: 0x0808220842002121, shift: 59, offset: 64 },
    MagicEntry { mask: Bitboard(0x0000004020100A00), magic: 0x8622020401E40388, shift: 59, offset: 96 },
    MagicEntry { mask: Bitboard(0x0000000040221400), magic: 0x20060A0200200803, shift: 59, offset: 128 },
    MagicEntry { mask: Bitboard(0x0000000002442800), magic: 0x24040520000D0001, shift: 59, offset: 160 },
    MagicEntry { mask: Bitboard(0x0000000204085000), magic: 0x0201100210000002, shift: 59, offset: 192 },
    MagicEntry { mask: Bitboard(0x0000020408102000), magic: 0x4028410820108002, shift: 59, offset: 224 },
    MagicEntry { mask: Bitboard(0x0002040810204000), magic: 0x0300818800822021, shift: 58, offset: 256 },
    MagicEntry { mask: Bitboard(0x0020100804020000), magic: 0x0000280208720401, shift: 59, offset: 320 },
    MagicEntry { mask: Bitboard(0x0040201008040000), magic: 0x4000200184028884, shift: 59, offset: 352 },
    MagicEntry { mask: Bitboard(0x00004020100A0000), magic: 0x0080900400404031, shift: 59, offset: 384 },
    MagicEntry { mask: Bitboard(0x0000004022140000), magic: 0x0150040400900202, shift: 59, offset: 416 },
    MagicEntry { mask: Bitboard(0x0000000244280000), magic: 0x000004102844A311, shift: 59, offset: 448 },
    MagicEntry { mask: Bitboard(0x0000020408500000), magic: 0x60A0020802080060, shift: 59, offset: 480 },
    MagicEntry { mask: Bitboard(0x0002040810200000), magic: 0x3000004304202040, shift: 59, offset: 512 },
    MagicEntry { mask: Bitboard(0x0004081020400000), magic: 0x0004020090841004, shift: 59, offset: 544 },
    MagicEntry { mask: Bitboard(0x0010080402000200), magic: 0x2240900C10049100, shift: 59, offset: 576 },
    MagicEntry { mask: Bitboard(0x0020100804000400), magic: 0x0002269004080880, shift: 59, offset: 608 },
    MagicEntry { mask: Bitboard(0x004020100A000A00), magic: 0x0030000800409220, shift: 57, offset: 640 },
    MagicEntry { mask: Bitboard(0x0000402214001400), magic: 0xC80807108200C008, shift: 57, offset: 768 },
    MagicEntry { mask: Bitboard(0x0000024428002800), magic: 0x0805021190400A00, shift: 57, offset: 896 },
    MagicEntry { mask: Bitboard(0x0002040850005000), magic: 0x7000802808042600, shift: 57, offset: 1024 },
    MagicEntry { mask: Bitboard(0x0004081020002000), magic: 0x101400C14C140400, shift: 59, offset: 1152 },
    MagicEntry { mask: Bitboard(0x0008102040004000), magic: 0x2809204504020208, shift: 59, offset: 1184 },
    MagicEntry { mask: Bitboard(0x0008040200020400), magic: 0x0004304040100100, shift: 59, offset: 1216 },
    MagicEntry { mask: Bitboard(0x0010080400040800), magic: 0x10440A0414081800, shift: 59, offset: 1248 },
    MagicEntry { mask: Bitboard(0x0020100A000A1000), magic: 0x8348040006040010, shift: 57, offset: 1280 },
    MagicEntry { mask: Bitboard(0x0040221400142200), magic: 0x0048080010820202, shift: 55, offset: 1408 },
    MagicEntry { mask: Bitboard(0x0002442800284400), magic: 0x9400840102802000, shift: 55, offset: 1920 },
    MagicEntry { mask: Bitboard(0x0004085000500800), magic: 0x0002820059080202, shift: 57, offset: 2432 },
    MagicEntry { mask: Bitboard(0x0008102000201000), magic: 0x00070A0401082101, shift: 59, offset: 2560 },
    MagicEntry { mask: Bitboard(0x0010204000402000), magic: 0x040180B503010806, shift: 59, offset: 2592 },
    MagicEntry { mask: Bitboard(0x0004020002040800), magic: 0x2008206400088800, shift: 59, offset: 2624 },
    MagicEntry { mask: Bitboard(0x0008040004081000), magic: 0x1086901000040400, shift: 59, offset: 2656 },
    MagicEntry { mask: Bitboard(0x00100A000A102000), magic: 0x40C1540200900980, shift: 57, offset: 2688 },
    MagicEntry { mask: Bitboard(0x0022140014224000), magic: 0x0040940100900900, shift: 55, offset: 2816 },
    MagicEntry { mask: Bitboard(0x0044280028440200), magic: 0x8040820A000C0108, shift: 55, offset: 3328 },
    MagicEntry { mask: Bitboard(0x0008500050080400), magic: 0x4210004200004500, shift: 57, offset: 3840 },
    MagicEntry { mask: Bitboard(0x0010200020100800), magic: 0x00081204008AC150, shift: 59, offset: 3968 },
    MagicEntry { mask: Bitboard(0x0020400040201000), magic: 0x8008020020198390, shift: 59, offset: 4000 },
    MagicEntry { mask: Bitboard(0x0002000204081000), magic: 0x001802D210402000, shift: 59, offset: 4032 },
    MagicEntry { mask: Bitboard(0x0004000408102000), magic: 0x084202072000E440, shift: 59, offset: 4064 },
    MagicEntry { mask: Bitboard(0x000A000A10204000), magic: 0x02501040B0000800, shift: 57, offset: 4096 },
    MagicEntry { mask: Bitboard(0x0014001422400000), magic: 0x0001042011089800, shift: 57, offset: 4224 },
    MagicEntry { mask: Bitboard(0x0028002844020000), magic: 0x0604024CA2000402, shift: 57, offset: 4352 },
    MagicEntry { mask: Bitboard(0x0050005008040200), magic: 0x084005C081000080, shift: 57, offset: 4480 },
    MagicEntry { mask: Bitboard(0x0020002010080400), magic: 0x000C2840C4002900, shift: 59, offset: 4608 },
    MagicEntry { mask: Bitboard(0x0040004020100800), magic: 0x2004080080324100, shift: 59, offset: 4640 },
    MagicEntry { mask: Bitboard(0x0000020408102000), magic: 0x0304040C24442410, shift: 59, offset: 4672 },
    MagicEntry { mask: Bitboard(0x0000040810204000), magic: 0x4200920811140020, shift: 59, offset: 4704 },
    MagicEntry { mask: Bitboard(0x00000A1020400000), magic: 0x4000A0212808210A, shift: 59, offset: 4736 },
    MagicEntry { mask: Bitboard(0x0000142240000000), magic: 0x2042024142022004, shift: 59, offset: 4768 },
    MagicEntry { mask: Bitboard(0x0000284402000000), magic: 0x0802248C10540885, shift: 59, offset: 4800 },
    MagicEntry { mask: Bitboard(0x0000500804020000), magic: 0x0842086028208008, shift: 59, offset: 4832 },
    MagicEntry { mask: Bitboard(0x0000201008040200), magic: 0x2004880888008381, shift: 59, offset: 4864 },
    MagicEntry { mask: Bitboard(0x0000402010080400), magic: 0x0406020405020000, shift: 59, offset: 4896 },
    MagicEntry { mask: Bitboard(0x0002040810204000), magic: 0x0300410809101200, shift: 58, offset: 4928 },
    MagicEntry { mask: Bitboard(0x0004081020400000), magic: 0x0001104104100200, shift: 59, offset: 4992 },
    MagicEntry { mask: Bitboard(0x000A102040000000), magic: 0x0020800100411020, shift: 59, offset: 5024 },
    MagicEntry { mask: Bitboard(0x0014224000000000), magic: 0x2224090000A08800, shift: 59, offset: 5056 },
    MagicEntry { mask: Bitboard(0x0028440200000000), magic: 0x2181020420342421, shift: 59, offset: 5088 },
    MagicEntry { mask: Bitboard(0x0050080402000000), magic: 0x0441220C04480200, shift: 59, offset: 5120 },
    MagicEntry { mask: Bitboard(0x0020100804020000), magic: 0x008010D010810040, shift: 59, offset: 5152 },
    MagicEntry { mask: Bitboard(0x0040201008040200), magic: 0x1010101004802048, shift: 58, offset: 5184 },
];
pub const BISHOP_TABLE_SIZE: usize = 5248;
#[rustfmt::skip]
pub const ROOK_MAGICS: &[MagicEntry; MAX_POS] = &[
    MagicEntry { mask: Bitboard(0x000101010101017E), magic: 0x0080001140028420, shift: 52, offset: 0 },
    MagicEntry { mask: Bitboard(0x000202020202027C), magic: 0x0040014020003006, shift: 53, offset: 4096 },
    MagicEntry { mask: Bitboard(0x000404040404047A), magic: 0x410010A001C10008, shift: 53, offset: 6144 },
    MagicEntry { mask: Bitboard(0x0008080808080876), magic: 0x0080100015280080, shift: 53, offset: 8192 },
    MagicEntry { mask: Bitboard(0x001010101010106E), magic: 0x8200084402002010, shift: 53, offset: 10240 },
    MagicEntry { mask: Bitboard(0x002020202020205E), magic: 0x0200060008100304, shift: 53, offset: 12288 },
    MagicEntry { mask: Bitboard(0x004040404040403E), magic: 0x09001200088B0014, shift: 53, offset: 14336 },
    MagicEntry { mask: Bitboard(0x008080808080807E), magic: 0x008000C021000080, shift: 52, offset: 16384 },
    MagicEntry { mask: Bitboard(0x0001010101017E00), magic: 0x1000802040008000, shift: 53, offset: 20480 },
    MagicEntry { mask: Bitboard(0x0002020202027C00), magic: 0x0102002601008044, shift: 54, offset: 22528 },
    MagicEntry { mask: Bitboard(0x0004040404047A00), magic: 0x800080200A801000, shift: 54, offset: 23552 },
    MagicEntry { mask: Bitboard(0x0008080808087600), magic: 0x1000804800900080, shift: 54, offset: 24576 },
    MagicEntry { mask: Bitboard(0x0010101010106E00), magic: 0x2480800800CC0080, shift: 54, offset: 25600 },
    MagicEntry { mask: Bitboard(0x0020202020205E00), magic: 0x2006800200802400, shift: 54, offset: 26624 },
    MagicEntry { mask: Bitboard(0x0040404040403E00), magic: 0x0004001008048201, shift: 54, offset: 27648 },
    MagicEntry { mask: Bitboard(0x0080808080807E00), magic: 0x0C188001000C5080, shift: 53, offset: 28672 },
    MagicEntry { mask: Bitboard(0x00010101017E0100), magic: 0x0401908000400024, shift: 53, offset: 30720 },
    MagicEntry { mask: Bitboard(0x00020202027C0200), magic: 0x000480802008400D, shift: 54, offset: 32768 },
    MagicEntry { mask: Bitboard(0x00040404047A0400), magic: 0x420A020041801022, shift: 54, offset: 33792 },
    MagicEntry { mask: Bitboard(0x0008080808760800), magic: 0x0014210008100102, shift: 54, offset: 34816 },
    MagicEntry { mask: Bitboard(0x00101010106E1000), magic: 0x4001010004100800, shift: 54, offset: 35840 },
    MagicEntry { mask: Bitboard(0x00202020205E2000), magic: 0x8200808044000200, shift: 54, offset: 36864 },
    MagicEntry { mask: Bitboard(0x00404040403E4000), magic: 0x0A00B40001100208, shift: 54, offset: 37888 },
    MagicEntry { mask: Bitboard(0x00808080807E8000), magic: 0x2043820004850844, shift: 53, offset: 38912 },
    MagicEntry { mask: Bitboard(0x000101017E010100), magic: 0x0000852080094000, shift: 53, offset: 40960 },
    MagicEntry { mask: Bitboard(0x000202027C020200), magic: 0x0430004040002010, shift: 54, offset: 43008 },
    MagicEntry { mask: Bitboard(0x000404047A040400), magic: 0x2220028080100128, shift: 54, offset: 44032 },
    MagicEntry { mask: Bitboard(0x0008080876080800), magic: 0x0004100080280080, shift: 54, offset: 45056 },
    MagicEntry { mask: Bitboard(0x001010106E101000), magic: 0x0020040080080080, shift: 54, offset: 46080 },
    MagicEntry { mask: Bitboard(0x002020205E202000), magic: 0x1302000200241008, shift: 54, offset: 47104 },
    MagicEntry { mask: Bitboard(0x004040403E404000), magic: 0x4801000101020004, shift: 54, offset: 48128 },
    MagicEntry { mask: Bitboard(0x008080807E808000), magic: 0x0002150200006084, shift: 53, offset: 49152 },
    MagicEntry { mask: Bitboard(0x0001017E01010100), magic: 0x00008040008000A0, shift: 53, offset: 51200 },
    MagicEntry { mask: Bitboard(0x0002027C02020200), magic: 0x0210002000C00042, shift: 54, offset: 53248 },
    MagicEntry { mask: Bitboard(0x0004047A04040400), magic: 0x0008310041002000, shift: 54, offset: 54272 },
    MagicEntry { mask: Bitboard(0x0008087608080800), magic: 0x4600852800801000, shift: 54, offset: 55296 },
    MagicEntry { mask: Bitboard(0x0010106E10101000), magic: 0x0000180080800400, shift: 54, offset: 56320 },
    MagicEntry { mask: Bitboard(0x0020205E20202000), magic: 0x40A600843A004810, shift: 54, offset: 57344 },
    MagicEntry { mask: Bitboard(0x0040403E40404000), magic: 0x0020481044000241, shift: 54, offset: 58368 },
    MagicEntry { mask: Bitboard(0x0080807E80808000), magic: 0x000480194A800300, shift: 53, offset: 59392 },
    MagicEntry { mask: Bitboard(0x00017E0101010100), magic: 0x0A10204000848010, shift: 53, offset: 61440 },
    MagicEntry { mask: Bitboard(0x00027C0202020200), magic: 0x02005000200E4000, shift: 54, offset: 63488 },
    MagicEntry { mask: Bitboard(0x00047A0404040400), magic: 0x0001100020008080, shift: 54, offset: 64512 },
    MagicEntry { mask: Bitboard(0x0008760808080800), magic: 0x40404200100A0020, shift: 54, offset: 65536 },
    MagicEntry { mask: Bitboard(0x00106E1010101000), magic: 0x024030880101000C, shift: 54, offset: 66560 },
    MagicEntry { mask: Bitboard(0x00205E2020202000), magic: 0x6005340002008080, shift: 54, offset: 67584 },
    MagicEntry { mask: Bitboard(0x00403E4040404000), magic: 0x1302D05201240008, shift: 54, offset: 68608 },
    MagicEntry { mask: Bitboard(0x00807E8080808000), magic: 0x08012040A1020004, shift: 53, offset: 69632 },
    MagicEntry { mask: Bitboard(0x007E010101010100), magic: 0x0C04A08002400480, shift: 53, offset: 71680 },
    MagicEntry { mask: Bitboard(0x007C020202020200), magic: 0x40150082C0006100, shift: 54, offset: 73728 },
    MagicEntry { mask: Bitboard(0x007A040404040400), magic: 0x0240312000450900, shift: 54, offset: 74752 },
    MagicEntry { mask: Bitboard(0x0076080808080800), magic: 0x050490002900A100, shift: 54, offset: 75776 },
    MagicEntry { mask: Bitboard(0x006E101010101000), magic: 0x0220640080480080, shift: 54, offset: 76800 },
    MagicEntry { mask: Bitboard(0x005E202020202000), magic: 0x3044000200800480, shift: 54, offset: 77824 },
    MagicEntry { mask: Bitboard(0x003E404040404000), magic: 0x0088100259080400, shift: 54, offset: 78848 },
    MagicEntry { mask: Bitboard(0x007E808080808000), magic: 0x000024C401148200, shift: 53, offset: 79872 },
    MagicEntry { mask: Bitboard(0x7E01010101010100), magic: 0x0002C18221D20102, shift: 52, offset: 81920 },
    MagicEntry { mask: Bitboard(0x7C02020202020200), magic: 0x48044202822302B2, shift: 53, offset: 86016 },
    MagicEntry { mask: Bitboard(0x7A04040404040400), magic: 0x00808010E0410A02, shift: 53, offset: 88064 },
    MagicEntry { mask: Bitboard(0x7608080808080800), magic: 0x8002004011182006, shift: 53, offset: 90112 },
    MagicEntry { mask: Bitboard(0x6E10101010101000), magic: 0x0022000820441002, shift: 53, offset: 92160 },
    MagicEntry { mask: Bitboard(0x5E20202020202000), magic: 0x0006000410010852, shift: 53, offset: 94208 },
    MagicEntry { mask: Bitboard(0x3E40404040404000), magic: 0x100200210400C802, shift: 53, offset: 96256 },
    MagicEntry { mask: Bitboard(0x7E80808080808000), magic: 0x0028005484010022, shift: 52, offset: 98304 },
];
pub const ROOK_TABLE_SIZE: usize = 102400;
