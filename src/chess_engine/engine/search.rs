//! The search: choosing the best move from a position.
//!
//! The driver, [`search_position`], runs *iterative deepening* — it searches to
//! depth 1, then 2, and so on — over a fail-soft [negamax] with alpha-beta
//! pruning. At the leaves a quiescence search resolves pending
//! captures and promotions so the static [evaluation](super::evaluation) is only
//! applied to quiet positions. Moves are ordered (promotions and MVV-LVA
//! captures first) to make alpha-beta prune more. Draws (fifty-move rule,
//! repetition, insufficient material) score `0`, and mates are encoded as
//! `MATE_SCORE - ply` so that shorter mates score higher.
//!
//! The search aborts cooperatively: every `ABORT_CHECK_INTERVAL` nodes it
//! polls a stop flag and an optional deadline, so the UCI layer can stop it
//! mid-search. The UCI layer owns time allocation; the search only obeys the
//! limits it is handed.
//!
//! [negamax]: https://www.chessprogramming.org/Negamax

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use crate::chess_engine::engine::evaluation::evaluate;
use crate::chess_engine::engine::transposition::{Bound, TranspositionTable};
use crate::chess_engine::piece::Piece;

use super::super::{board::Board, moves::Move, moves::SpecialMove};

/// Score assigned to checkmate at the root; a mate `n` plies away scores
/// `MATE_SCORE - n`, so faster mates are preferred.
pub const MATE_SCORE: i32 = 30_000;
/// Scores above this magnitude encode a forced mate.
pub const MATE_THRESHOLD: i32 = MATE_SCORE - 1_000;
const INFINITY: i32 = i32::MAX - 1;

/// How often (in nodes) the abort conditions are polled.
const ABORT_CHECK_INTERVAL: u64 = 2048;

/// Maximum iterative-deepening depth the search will attempt.
pub const MAX_DEPTH: u8 = 64;

/// The conditions under which a search stops.
#[derive(Debug, Clone, Copy)]
pub struct SearchLimits {
    /// Maximum iterative-deepening depth.
    pub depth: u8,
    /// Hard wall-clock limit for the whole search, if any.
    pub deadline: Option<Instant>,
}

impl SearchLimits {
    /// Limits the search to a fixed depth (clamped to [`MAX_DEPTH`]) with no
    /// time limit.
    #[must_use]
    pub fn depth(depth: u8) -> Self {
        Self {
            depth: depth.min(MAX_DEPTH),
            deadline: None,
        }
    }

    /// An unbounded search (to [`MAX_DEPTH`], no deadline); stopped only via the
    /// stop flag.
    #[must_use]
    pub const fn infinite() -> Self {
        Self {
            depth: MAX_DEPTH,
            deadline: None,
        }
    }
}

/// The outcome of a search: the chosen move and the statistics behind it.
#[derive(Debug)]
pub struct SearchResult {
    /// The best move found, or `None` if the search was aborted before
    /// completing even depth 1 (e.g. no legal moves, or an immediate stop).
    pub best_move: Option<Move>,
    /// Score of the position from the side-to-move's perspective, in centipawns
    /// (or a mate score; see [`MATE_SCORE`]).
    pub score: i32,
    /// The depth of the last fully completed iteration.
    pub depth: u8,
    /// Total nodes visited across all iterations.
    pub nodes: u64,
    /// The principal variation: the expected line of best play, `best_move`
    /// first.
    pub pv: Vec<Move>,
}

/// Mutable state threaded through the recursive search: the stop signal, the
/// deadline, this worker's local node count, the shared cross-thread node total,
/// the transposition table, and whether an abort has been requested.
struct SearchContext<'a> {
    stop: &'a AtomicBool,
    deadline: Option<Instant>,
    nodes: u64,
    aborted: bool,
    tt: &'a TranspositionTable,
    /// Aggregate node count across all search threads, flushed in bulk so the
    /// hot path stays contention-free.
    shared_nodes: &'a AtomicU64,
    /// Per-thread seed that perturbs the ordering of equal-ranked quiet moves so
    /// Lazy SMP helpers explore divergent trees. `0` (the main worker) keeps the
    /// clean, deterministic order.
    order_noise: u64,
}

impl SearchContext<'_> {
    /// Counts the current node and, every [`ABORT_CHECK_INTERVAL`] nodes, flushes
    /// the interval into the shared total and checks the stop flag and deadline.
    /// Returns `true` once an abort has been triggered.
    fn count_node_and_check_abort(&mut self) -> bool {
        self.nodes += 1;
        if self.nodes.is_multiple_of(ABORT_CHECK_INTERVAL) {
            self.shared_nodes
                .fetch_add(ABORT_CHECK_INTERVAL, Ordering::Relaxed);
            if self.stop.load(Ordering::Relaxed) {
                self.aborted = true;
            } else if let Some(deadline) = self.deadline
                && Instant::now() >= deadline
            {
                self.aborted = true;
            }
        }
        self.aborted
    }
}

/// Backwards-compatible fixed-depth entry point: searches a clone of `board` to
/// the given depth with no time limit.
///
/// ```
/// use chess_engine::chess_engine::board::Board;
/// use chess_engine::chess_engine::engine::search::{find_best_move, MATE_THRESHOLD};
/// use chess_engine::chess_engine::utils::init_tables;
///
/// init_tables();
/// // White to move and mate in one: Ra8#.
/// let board = Board::from_fen("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1").unwrap();
/// let result = find_best_move(&board, 2);
/// assert_eq!(result.best_move.unwrap().to_string(), "a1a8");
/// assert!(result.score >= MATE_THRESHOLD); // a forced mate was found
/// ```
#[must_use]
pub fn find_best_move(board: &Board, depth: u8) -> SearchResult {
    let stop = AtomicBool::new(false);
    // No shared table here, so allocate a private one for this search.
    let tt = TranspositionTable::new();
    search_position(
        &mut board.clone(),
        SearchLimits::depth(depth),
        &stop,
        &tt,
        1,
        false,
    )
}

/// Iterative-deepening, alpha-beta search over `threads` workers (Lazy SMP).
///
/// One *main* worker drives reporting and supplies the returned result; the
/// remaining `threads - 1` *helper* workers search the same root on their own
/// board clones and share the transposition table, diverging through TT timing
/// races so they widen the main worker's effective search. All workers share the
/// stop flag, the deadline, and an aggregate node counter; each completed search
/// returns the result of the main worker's last fully completed iteration.
///
/// `threads` is clamped to at least 1. When `report` is set, a UCI `info` line is
/// printed per depth by the main worker.
pub fn search_position(
    board: &mut Board,
    limits: SearchLimits,
    stop: &AtomicBool,
    tt: &TranspositionTable,
    threads: usize,
    report: bool,
) -> SearchResult {
    let start = Instant::now();
    tt.new_generation();
    let shared_nodes = AtomicU64::new(0);

    // `order_noise` of 0 marks the main worker (clean, deterministic ordering);
    // helpers get distinct non-zero seeds so they diverge.
    let make_ctx = |order_noise: u64| SearchContext {
        stop,
        deadline: limits.deadline,
        nodes: 0,
        aborted: false,
        tt,
        shared_nodes: &shared_nodes,
        order_noise,
    };

    if threads <= 1 {
        return run_iterative(board, limits, &mut make_ctx(0), start, report, 1);
    }

    std::thread::scope(|scope| {
        // Helper workers: own board clone, no reporting, a per-thread ordering
        // seed, and a slightly staggered start depth. Their results are discarded;
        // they contribute only through the shared transposition table.
        for i in 1..threads {
            let mut helper_board = board.clone();
            let mut ctx = make_ctx(i as u64);
            let start_depth = if i % 2 == 0 { 1 } else { 2 };
            scope.spawn(move || {
                run_iterative(
                    &mut helper_board,
                    limits,
                    &mut ctx,
                    start,
                    false,
                    start_depth,
                );
            });
        }

        // Main worker runs on this thread and owns the reported result.
        let result = run_iterative(board, limits, &mut make_ctx(0), start, report, 1);
        // Tell the helpers to wind down; `scope` then joins them.
        stop.store(true, Ordering::Relaxed);
        result
    })
}

/// One worker's iterative-deepening loop: searches depths `start_depth..` until
/// the depth limit, deadline, stop flag, or a forced mate ends it, returning the
/// last fully completed iteration. Node statistics reflect the shared
/// cross-thread total.
fn run_iterative(
    board: &mut Board,
    limits: SearchLimits,
    ctx: &mut SearchContext,
    start: Instant,
    report: bool,
    start_depth: u8,
) -> SearchResult {
    let mut result = SearchResult {
        best_move: None,
        score: 0,
        depth: 0,
        nodes: 0,
        pv: Vec::new(),
    };

    for depth in start_depth..=limits.depth.max(start_depth) {
        let mut pv = Vec::new();
        let score = negamax(board, depth, 0, -INFINITY, INFINITY, ctx, &mut pv);
        if ctx.aborted {
            break;
        }

        result = SearchResult {
            best_move: pv.first().copied(),
            score,
            depth,
            nodes: ctx.shared_nodes.load(Ordering::Relaxed),
            pv,
        };

        if report {
            print_info(&result, start);
        }

        // a forced mate was found; deeper iterations cannot improve it
        if score.abs() >= MATE_THRESHOLD {
            break;
        }
    }

    result.nodes = ctx.shared_nodes.load(Ordering::Relaxed);
    result
}

/// Prints a UCI `info` line for a completed depth (score in centipawns or
/// `mate N`, plus nodes, time, and the principal variation).
fn print_info(result: &SearchResult, start: Instant) {
    let elapsed = start.elapsed();
    let millis = elapsed.as_millis().max(1);
    let nps = (u128::from(result.nodes) * 1000) / millis;

    let score = if result.score.abs() >= MATE_THRESHOLD {
        // moves (not plies) until mate, negative when we are getting mated
        let plies = MATE_SCORE - result.score.abs();
        let moves = (plies + 1) / 2;
        format!("mate {}", if result.score > 0 { moves } else { -moves })
    } else {
        format!("cp {}", result.score)
    };

    let pv: Vec<String> = result
        .pv
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    println!(
        "info depth {} score {} nodes {} time {} nps {} pv {}",
        result.depth,
        score,
        result.nodes,
        millis,
        nps,
        pv.join(" ")
    );
}

/// Negamax with alpha-beta pruning (fail-soft). Returns the score from the
/// perspective of the side to move; `pv` receives the principal variation.
#[allow(clippy::cast_possible_truncation)]
fn negamax(
    board: &mut Board,
    depth: u8,
    ply: u8,
    mut alpha: i32,
    beta: i32,
    ctx: &mut SearchContext,
    pv: &mut Vec<Move>,
) -> i32 {
    if ctx.count_node_and_check_abort() {
        return 0;
    }

    // draw by fifty-move rule, insufficient material, or repetition (twofold
    // is enough inside the search: if a position repeats once, best play can
    // force the threefold)
    if ply > 0
        && (board.halfmove_count >= 100
            || board.is_insufficient_material()
            || board.get_count_of_current_position_reached() >= 1)
    {
        return 0;
    }

    if depth == 0 {
        return quiescence(board, alpha, beta, ctx);
    }

    // Transposition-table probe. A sufficiently deep entry can cut the node off
    // outright; otherwise its move still seeds move ordering. The root (ply 0) is
    // never cut off, so its move loop always runs and yields a best move.
    let tt_entry = ctx.tt.probe(board.zobrist_key);
    if ply > 0
        && let Some(entry) = tt_entry
        && entry.depth >= depth
    {
        let score = score_from_tt(entry.score, ply);
        match entry.bound {
            Bound::Exact => return score,
            Bound::Lower if score >= beta => return score,
            Bound::Upper if score <= alpha => return score,
            _ => {}
        }
    }
    let tt_move = tt_entry.and_then(|entry| entry.mv);

    let mut moves = board.generate_moves(board.turn);
    if moves.is_empty() {
        return if board.in_check(board.turn) {
            // mated: worse the closer to the root it happens
            -(MATE_SCORE - i32::from(ply))
        } else {
            0 // stalemate
        };
    }
    order_moves(board, &mut moves, tt_move, ctx.order_noise);

    let alpha_orig = alpha;
    let mut best_score = -INFINITY;
    let mut best_move: Option<Move> = None;
    for mv in moves {
        board.commit_verified_move(mv);
        let mut child_pv = Vec::new();
        let score = -negamax(board, depth - 1, ply + 1, -beta, -alpha, ctx, &mut child_pv);
        board.unmake_move();

        if ctx.aborted {
            return 0;
        }

        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
        if score > alpha {
            alpha = score;
            pv.clear();
            pv.push(mv);
            pv.append(&mut child_pv);
        }
        if alpha >= beta {
            break; // beta cutoff
        }
    }

    // Classify the result relative to the original window and cache it.
    let bound = if best_score <= alpha_orig {
        Bound::Upper
    } else if best_score >= beta {
        Bound::Lower
    } else {
        Bound::Exact
    };
    ctx.tt.store(
        board.zobrist_key,
        best_move,
        score_to_tt(best_score, ply) as i16,
        depth,
        bound,
    );

    best_score
}

/// Rebases a mate score from node-relative (as stored in the table) to
/// root-relative (as used in the search), undoing [`score_to_tt`].
fn score_from_tt(score: i32, ply: u8) -> i32 {
    if score >= MATE_THRESHOLD {
        score - i32::from(ply)
    } else if score <= -MATE_THRESHOLD {
        score + i32::from(ply)
    } else {
        score
    }
}

/// Rebases a mate score from root-relative to node-relative for storage, so a
/// "mate in N from here" is cached independently of how deep this node sits.
fn score_to_tt(score: i32, ply: u8) -> i32 {
    if score >= MATE_THRESHOLD {
        score + i32::from(ply)
    } else if score <= -MATE_THRESHOLD {
        score - i32::from(ply)
    } else {
        score
    }
}

/// Quiescence search: extend the search through captures and promotions so
/// that the static evaluation is only applied to quiet positions.
fn quiescence(board: &mut Board, mut alpha: i32, beta: i32, ctx: &mut SearchContext) -> i32 {
    if ctx.count_node_and_check_abort() {
        return 0;
    }

    let stand_pat = evaluate(board);
    if stand_pat >= beta {
        return beta;
    }
    if stand_pat > alpha {
        alpha = stand_pat;
    }

    let mut moves: Vec<Move> = board
        .generate_moves(board.turn)
        .into_iter()
        .filter(|&m| is_tactical(board, m))
        .collect();
    order_moves(board, &mut moves, None, ctx.order_noise);

    for mv in moves {
        board.commit_verified_move(mv);
        let score = -quiescence(board, -beta, -alpha, ctx);
        board.unmake_move();

        if ctx.aborted {
            return 0;
        }

        if score >= beta {
            return beta;
        }
        if score > alpha {
            alpha = score;
        }
    }

    alpha
}

/// Returns `true` if `mv` is a capture, promotion, or en passant — the moves
/// the quiescence search extends through.
fn is_tactical(board: &Board, mv: Move) -> bool {
    match mv.get_special_move() {
        SpecialMove::Promotion | SpecialMove::EnPassant => true,
        SpecialMove::Castle => false,
        SpecialMove::NormalMove => !board.empty_tiles.is_square_set(mv.get_dest().into()),
    }
}

/// Material values (in centipawns) used only for move ordering; the positional
/// [evaluation](super::evaluation) uses its own scale.
const fn piece_value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => 100,
        Piece::Knight => 320,
        Piece::Bishop => 330,
        Piece::Rook => 500,
        Piece::Queen => 900,
        Piece::King => 10_000,
        Piece::None => 0,
    }
}

/// Score given to the transposition-table move so it is always searched first.
const TT_MOVE_SCORE: i32 = 1_000_000;

/// Order moves so the most forcing ones are searched first: the
/// transposition-table move, then promotions and captures (MVV-LVA: most
/// valuable victim, least valuable attacker), then quiet moves. A non-zero
/// `noise` seed perturbs the order of equal-ranked (quiet) moves so Lazy SMP
/// helpers diverge.
fn order_moves(board: &Board, moves: &mut [Move], tt_move: Option<Move>, noise: u64) {
    let mut scored: Vec<(i32, Move)> = moves
        .iter()
        .map(|&mv| {
            (
                move_order_score(board, mv, tt_move) + order_jitter(noise, mv),
                mv,
            )
        })
        .collect();
    scored.sort_by_key(|(score, _)| -*score);
    for (slot, (_, mv)) in moves.iter_mut().zip(scored) {
        *slot = mv;
    }
}

/// A small (`0..64`) per-thread perturbation, derived from the worker's `noise`
/// seed and the move. It is smaller than the gap between any two move-ordering
/// tiers, so it only ever shuffles already-equal (quiet) moves and never demotes
/// a capture or the TT move. Returns `0` for the main worker (`noise == 0`).
#[allow(clippy::cast_possible_truncation)]
fn order_jitter(noise: u64, mv: Move) -> i32 {
    if noise == 0 {
        return 0;
    }
    let hash = (noise ^ u64::from(mv.get_raw())).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    (hash >> 58) as i32
}

/// Heuristic ordering score for a single move: the transposition-table move
/// highest, then promotions, then captures by MVV-LVA (victim value weighted
/// above attacker value), then quiet moves at `0`.
fn move_order_score(board: &Board, mv: Move, tt_move: Option<Move>) -> i32 {
    if tt_move == Some(mv) {
        return TT_MOVE_SCORE;
    }
    let mut score = 0;
    match mv.get_special_move() {
        SpecialMove::Promotion => {
            score += 10_000 + piece_value(mv.get_promotion());
        }
        SpecialMove::EnPassant => {
            return 8_000 + piece_value(Piece::Pawn) * 10 - piece_value(Piece::Pawn);
        }
        _ => (),
    }
    let victim = board.get_piece_type_containing_position(mv.get_dest());
    if victim != Piece::None {
        let attacker = board.get_piece_type_containing_position(mv.get_origin());
        score += 8_000 + piece_value(victim) * 10 - piece_value(attacker);
    }
    score
}
