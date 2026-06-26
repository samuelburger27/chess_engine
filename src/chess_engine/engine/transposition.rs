//! A lock-free transposition table: a fixed-size cache of search results keyed
//! by the board's Zobrist hash.
//!
//! Each searched position records its score, the depth it was searched to, a
//! [`Bound`] describing whether that score is exact or only a bound (from an
//! alpha-beta cutoff), and the best move found. Re-reaching the same position —
//! within one search, a later iterative-deepening iteration, or a later move in
//! the game — can then reuse the result to cut the subtree off, and can always
//! reuse the stored move to improve move ordering.
//!
//! # Concurrency
//!
//! The table is shared (via `Arc`) between the search thread and the UCI thread,
//! which may [`clear`](TranspositionTable::clear) it between games. Rather than
//! lock, each slot is two [`AtomicU64`]s storing `key ^ data` and `data`; a
//! reader accepts an entry only when `stored_key ^ data` reproduces the probed
//! key. A torn read from a concurrent writer fails that check and is treated as
//! a miss. This is Hyatt's lockless scheme.
//!
//! # Known limitations
//!
//! A cached score ignores the path used to reach the position, so a score that
//! depended on the repetition/fifty-move history can be slightly inaccurate. The
//! search runs its own draw checks before probing, so the current node's own
//! repetition is still detected; deeper path effects are not. This is the
//! standard, accepted trade-off.

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

use crate::chess_engine::moves::Move;

/// Number of slots in the table. `2^22` slots × 16 bytes/slot = 64 MiB. A power
/// of two so the index is a cheap `key & MASK`.
const TABLE_SIZE: usize = 1 << 22;
/// Mask turning a Zobrist key into a slot index.
const INDEX_MASK: u64 = (TABLE_SIZE as u64) - 1;

/// Whether a stored score is exact or merely a bound from an alpha-beta cutoff.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bound {
    /// The score is exact (a PV node: `alpha < score < beta`).
    Exact,
    /// A lower bound: the search failed high (`score >= beta`); the true score is
    /// at least this.
    Lower,
    /// An upper bound: the search failed low (`score <= alpha`); the true score is
    /// at most this.
    Upper,
}

impl Bound {
    /// Packs into the 2 bits used in a slot's data word.
    const fn to_bits(self) -> u64 {
        match self {
            Self::Exact => 0,
            Self::Lower => 1,
            Self::Upper => 2,
        }
    }

    /// Inverse of [`Bound::to_bits`].
    const fn from_bits(bits: u64) -> Self {
        match bits & 0b11 {
            1 => Self::Lower,
            2 => Self::Upper,
            _ => Self::Exact,
        }
    }
}

/// The decoded contents of a transposition-table hit.
#[derive(Clone, Copy, Debug)]
pub struct TtData {
    /// The best move recorded for the position, if one was stored.
    pub mv: Option<Move>,
    /// The stored score (centipawns or a mate score), from the side-to-move's
    /// perspective. Mate-distance correction is the search's responsibility.
    pub score: i32,
    /// The depth to which the position was searched.
    pub depth: u8,
    /// Whether [`score`](Self::score) is exact or a bound.
    pub bound: Bound,
}

/// One table slot: `key = zobrist ^ data` and the packed `data` word. An empty
/// slot is `(0, 0)`.
struct Slot {
    key: AtomicU64,
    data: AtomicU64,
}

impl Slot {
    const fn empty() -> Self {
        Self {
            key: AtomicU64::new(0),
            data: AtomicU64::new(0),
        }
    }
}

// Data-word bit layout:
//   bits  0–15: move (raw u16; 0 = no move)
//   bits 16–31: score as i16 reinterpreted as u16
//   bits 32–39: depth (u8)
//   bits 40–41: bound
//   bits 42–49: generation (u8)
const DEPTH_SHIFT: u64 = 32;
const BOUND_SHIFT: u64 = 40;
const GEN_SHIFT: u64 = 42;

/// Packs the fields of an entry into a single 64-bit data word.
fn pack(mv: Option<Move>, score: i16, depth: u8, bound: Bound, generation: u8) -> u64 {
    let mv_bits = u64::from(mv.map_or(0, |m| m.get_raw()));
    let score_bits = u64::from(score.cast_unsigned());
    mv_bits
        | (score_bits << 16)
        | (u64::from(depth) << DEPTH_SHIFT)
        | (bound.to_bits() << BOUND_SHIFT)
        | (u64::from(generation) << GEN_SHIFT)
}

/// Decodes a data word's depth (for the replacement heuristic).
#[allow(clippy::cast_possible_truncation)]
const fn unpack_depth(data: u64) -> u8 {
    (data >> DEPTH_SHIFT) as u8
}

/// Decodes a data word's generation (for the replacement heuristic).
#[allow(clippy::cast_possible_truncation)]
const fn unpack_generation(data: u64) -> u8 {
    (data >> GEN_SHIFT) as u8
}

/// A fixed-size, lock-free transposition table.
pub struct TranspositionTable {
    slots: Vec<Slot>,
    /// Bumped once per search so entries from earlier searches can be preferred
    /// for replacement.
    generation: AtomicU8,
}

impl TranspositionTable {
    /// Allocates a fresh 64 MiB table with all slots empty.
    #[must_use]
    pub fn new() -> Self {
        let slots = (0..TABLE_SIZE).map(|_| Slot::empty()).collect();
        Self {
            slots,
            generation: AtomicU8::new(0),
        }
    }

    /// Empties every slot. Safe to call while a search holds a shared reference;
    /// intended for `ucinewgame`.
    pub fn clear(&self) {
        for slot in &self.slots {
            slot.key.store(0, Ordering::Relaxed);
            slot.data.store(0, Ordering::Relaxed);
        }
        self.generation.store(0, Ordering::Relaxed);
    }

    /// Advances the generation counter; call once at the start of each search.
    pub fn new_generation(&self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(clippy::cast_possible_truncation)]
    const fn index(key: u64) -> usize {
        (key & INDEX_MASK) as usize
    }

    /// Looks up `key`. Returns `None` on a miss or a detected torn read.
    #[allow(clippy::cast_possible_truncation)]
    #[must_use]
    pub fn probe(&self, key: u64) -> Option<TtData> {
        let slot = &self.slots[Self::index(key)];
        let data = slot.data.load(Ordering::Relaxed);
        let stored_key = slot.key.load(Ordering::Relaxed);
        // Lockless validation: a consistent entry satisfies stored_key ^ data == key.
        if stored_key ^ data != key {
            return None;
        }
        let mv_raw = data as u16;
        let mv = if mv_raw == 0 {
            None
        } else {
            Some(Move::make_raw(mv_raw))
        };
        let score = i32::from(((data >> 16) as u16).cast_signed());
        Some(TtData {
            mv,
            score,
            depth: unpack_depth(data),
            bound: Bound::from_bits(data >> BOUND_SHIFT),
        })
    }

    /// Records the result of searching the position with the given `key`.
    ///
    /// Uses a depth-preferred replacement policy with aging: an existing entry is
    /// overwritten only if it is empty, comes from an earlier generation, or was
    /// searched no deeper than this one.
    pub fn store(&self, key: u64, mv: Option<Move>, score: i16, depth: u8, bound: Bound) {
        let slot = &self.slots[Self::index(key)];
        let generation = self.generation.load(Ordering::Relaxed);

        let existing = slot.data.load(Ordering::Relaxed);
        let replace = existing == 0
            || unpack_generation(existing) != generation
            || depth >= unpack_depth(existing);
        if !replace {
            return;
        }

        let data = pack(mv, score, depth, bound, generation);
        // Store data first, then key = zobrist ^ data, so a concurrent reader that
        // sees the new key also sees the matching data (and otherwise fails the
        // XOR check).
        slot.data.store(data, Ordering::Relaxed);
        slot.key.store(key ^ data, Ordering::Relaxed);
    }
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_engine::position::Position;

    fn sample_move() -> Move {
        Move::new_default(Position::new(12), Position::new(28)) // e2e4
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn pack_unpack_round_trips() {
        for &(score, bound) in &[
            (0_i16, Bound::Exact),
            (-1234, Bound::Upper),
            (29_001, Bound::Lower),
            (-29_001, Bound::Exact),
            (i16::MIN, Bound::Lower),
            (i16::MAX, Bound::Upper),
        ] {
            let data = pack(Some(sample_move()), score, 17, bound, 5);
            assert_eq!(data as u16, sample_move().get_raw());
            assert_eq!(i32::from(((data >> 16) as u16).cast_signed()), i32::from(score));
            assert_eq!(unpack_depth(data), 17);
            assert_eq!(Bound::from_bits(data >> BOUND_SHIFT), bound);
            assert_eq!(unpack_generation(data), 5);
        }
    }

    #[test]
    fn store_then_probe_returns_entry() {
        let tt = TranspositionTable::new();
        let key = 0xDEAD_BEEF_1234_5678;
        tt.store(key, Some(sample_move()), -29_001, 9, Bound::Lower);

        let got = tt.probe(key).expect("entry should be present");
        assert_eq!(got.mv, Some(sample_move()));
        assert_eq!(got.score, -29_001);
        assert_eq!(got.depth, 9);
        assert_eq!(got.bound, Bound::Lower);
    }

    #[test]
    fn probe_absent_key_misses() {
        let tt = TranspositionTable::new();
        tt.store(0xAAAA_AAAA_AAAA_AAAA, Some(sample_move()), 100, 5, Bound::Exact);
        // Same slot index, different key — must not be reported as a hit.
        let colliding = 0xAAAA_AAAA_AAAA_AAAA ^ (1 << 40);
        assert!(tt.probe(colliding).is_none());
        assert!(tt.probe(0x1111_2222_3333_4444).is_none());
    }

    #[test]
    fn clear_evicts_entries() {
        let tt = TranspositionTable::new();
        let key = 0x0102_0304_0506_0708;
        tt.store(key, None, 42, 3, Bound::Exact);
        assert!(tt.probe(key).is_some());
        tt.clear();
        assert!(tt.probe(key).is_none());
    }

    #[test]
    fn no_move_stored_as_none() {
        let tt = TranspositionTable::new();
        let key = 0x9999_8888_7777_6666;
        tt.store(key, None, 7, 4, Bound::Upper);
        let got = tt.probe(key).unwrap();
        assert_eq!(got.mv, None);
    }
}
