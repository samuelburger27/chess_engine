//! The search: choosing the best move from a position.
//!
//! The driver, [`search_position`], runs *iterative deepening* — it searches to
//! depth 1, then 2, and so on — over a fail-soft [negamax] with alpha-beta
//! pruning. Each iteration opens an *aspiration window* around the previous
//! score, widening it on a fail. Inside the tree the search prunes and reduces:
//! *null-move pruning* (a reduced search after passing the turn that still
//! fails high cuts the node), *principal variation search* (only the first
//! move gets a full window; the rest are probed with a zero-width window), and
//! *late move reductions* (late quiet moves are probed a ply shallower and
//! re-searched only on a fail-high). A node in check is extended one ply.
//!
//! At the leaves a quiescence search resolves pending
//! captures and promotions so the static [evaluation](super::evaluation) is only
//! applied to quiet positions; while in check it searches every evasion
//! instead, and hopeless captures are skipped by *delta pruning* and a
//! negative *static exchange evaluation* ([`see`]). Moves are ordered to make
//! alpha-beta prune more: the
//! transposition-table move, promotions and MVV-LVA captures, then quiet moves
//! led by the *killer moves* (quiet refutations of sibling nodes) and ranked by
//! the *history heuristic* (how often a move's origin→destination caused
//! cutoffs). Draws (fifty-move rule,
//! repetition, insufficient material) score `0`, and mates are encoded as
//! `MATE_SCORE - ply` so that shorter mates score higher.
//!
//! The search aborts cooperatively: every `ABORT_CHECK_INTERVAL` nodes it
//! polls a stop flag and an optional deadline, so the UCI layer can stop it
//! mid-search. The UCI layer owns time allocation; the search only obeys the
//! limits it is handed.
//!
//! [negamax]: https://www.chessprogramming.org/Negamax

use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use crate::chess_engine::bitboard::Bitboard;
use crate::chess_engine::board::{BLACK, Turn, WHITE};
use crate::chess_engine::computed_boards::{
    BISHOP_ATTACKS, BISHOP_BLOCKERS, BISHOP_MAGICS, KING_RING_MOVES, KNIGHT_MOVES, PAWN_ATTACKS,
    ROOK_ATTACKS, ROOK_BLOCKERS, ROOK_MAGICS,
};
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

/// Minimum depth at which null-move pruning is tried.
const NULL_MOVE_MIN_DEPTH: u8 = 3;
/// Minimum depth at which late move reductions apply.
const LMR_MIN_DEPTH: u8 = 3;
/// Number of moves searched at full depth before reductions kick in.
const LMR_MIN_MOVE_INDEX: usize = 3;
/// Half-width of the initial aspiration window around the previous
/// iteration's score.
const ASPIRATION_WINDOW: i32 = 40;
/// Iteration depth from which aspiration windows are used (earlier scores are
/// too unstable to centre a narrow window on).
const ASPIRATION_MIN_DEPTH: u8 = 4;
/// Once the window has been widened past this margin, re-search with the full
/// window instead of widening again.
const ASPIRATION_MAX_MARGIN: i32 = 640;

/// Maximum iterative-deepening depth the search will attempt.
pub const MAX_DEPTH: u8 = 64;

/// Hard cap on the distance from the root, including check extensions; a node
/// this deep returns its static evaluation instead of recursing further.
const MAX_PLY: usize = 128;

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
/// the transposition table, the quiet-move ordering heuristics, and whether an
/// abort has been requested.
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
    /// Killer moves: per ply, the last two quiet moves that caused a beta
    /// cutoff. Quiet moves that refuted a sibling often refute here too.
    killers: [[Option<Move>; 2]; MAX_PLY],
    /// History heuristic: for each (origin, destination) pair, how often quiet
    /// moves with that geometry caused beta cutoffs, weighted by depth².
    history: Box<[[i32; 64]; 64]>,
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

    /// Records a quiet move that caused a beta cutoff: it becomes the ply's
    /// first killer, and its origin→destination history weight grows by depth²
    /// (deeper refutations are stronger evidence).
    fn record_quiet_cutoff(&mut self, mv: Move, ply: u8, depth: u8) {
        let slot = &mut self.killers[usize::from(ply)];
        if slot[0] != Some(mv) {
            slot[1] = slot[0];
            slot[0] = Some(mv);
        }
        let (origin, dest) = mv.get_org_and_dest();
        let entry = &mut self.history[origin.as_usize()][dest.as_usize()];
        *entry = (*entry + i32::from(depth) * i32::from(depth)).min(MAX_HISTORY_SCORE);
    }
}

/// Backwards-compatible fixed-depth entry point: searches a clone of `board` to
/// the given depth with no time limit.
///
/// ```
/// use sabertooth::chess_engine::board::Board;
/// use sabertooth::chess_engine::engine::search::{find_best_move, MATE_THRESHOLD};
/// use sabertooth::chess_engine::utils::init_tables;
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
    /// A process-wide table reused across `find_best_move` calls, so repeated
    /// fixed-depth searches (e.g. the WAC suite's 300 positions) don't each
    /// allocate and zero a fresh 64 MiB table. Cleared at the start of every
    /// call to keep each search self-contained and deterministic.
    static TT: LazyLock<TranspositionTable> = LazyLock::new(TranspositionTable::new);

    let stop = AtomicBool::new(false);
    TT.clear();
    search_position(
        &mut board.clone(),
        SearchLimits::depth(depth),
        &stop,
        &TT,
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
        killers: [[None; 2]; MAX_PLY],
        history: Box::new([[0; 64]; 64]),
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

    let mut prev_score = 0;
    for depth in start_depth..=limits.depth.max(start_depth) {
        let mut pv = Vec::new();

        // Aspiration window: centre a narrow window on the last iteration's
        // score; on a fail outside it, widen the failing side and re-search.
        let mut margin = ASPIRATION_WINDOW;
        let (mut alpha, mut beta) = if depth >= ASPIRATION_MIN_DEPTH {
            (prev_score - margin, prev_score + margin)
        } else {
            (-INFINITY, INFINITY)
        };
        let score = loop {
            pv.clear();
            let score = negamax(board, depth, 0, alpha, beta, true, ctx, &mut pv);
            if ctx.aborted || (score > alpha && score < beta) {
                break score;
            }
            margin = margin.saturating_mul(4);
            if score <= alpha {
                alpha = if margin > ASPIRATION_MAX_MARGIN {
                    -INFINITY
                } else {
                    score - margin
                };
            } else {
                beta = if margin > ASPIRATION_MAX_MARGIN {
                    INFINITY
                } else {
                    score + margin
                };
            }
        };
        if ctx.aborted {
            break;
        }
        prev_score = score;

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
///
/// `null_allowed` gates null-move pruning so two null moves are never played
/// in a row (the null search itself passes `false`).
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]
fn negamax(
    board: &mut Board,
    depth: u8,
    ply: u8,
    mut alpha: i32,
    beta: i32,
    null_allowed: bool,
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

    if usize::from(ply) >= MAX_PLY {
        return evaluate(board);
    }

    let in_check = board.in_check(board.turn);
    // Check extension: never drop into quiescence (or shed depth) while in
    // check — evasions are forced and the tactics are still unresolved.
    let depth = if in_check { depth + 1 } else { depth };

    if depth == 0 {
        return quiescence(board, ply, alpha, beta, ctx);
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

    // Null-move pruning: if passing the turn (a move worse than every real
    // move, zugzwang aside) still fails high on a reduced search, the real
    // search would too — cut off. Skipped in check (the null move would be
    // illegal), near mate scores, and without non-pawn material (zugzwang).
    if null_allowed
        && ply > 0
        && depth >= NULL_MOVE_MIN_DEPTH
        && !in_check
        && beta.abs() < MATE_THRESHOLD
        && has_non_pawn_material(board)
    {
        let r = if depth >= 6 { 3 } else { 2 };
        board.make_null_move();
        let mut null_pv = Vec::new();
        let score = -negamax(
            board,
            depth - 1 - r,
            ply + 1,
            -beta,
            -beta + 1,
            false,
            ctx,
            &mut null_pv,
        );
        board.unmake_null_move();
        if ctx.aborted {
            return 0;
        }
        if score >= beta {
            // a mate score from the reduced null search is unproven
            return if score >= MATE_THRESHOLD { beta } else { score };
        }
    }

    let mut moves = board.generate_moves(board.turn);
    if moves.is_empty() {
        return if in_check {
            // mated: worse the closer to the root it happens
            -(MATE_SCORE - i32::from(ply))
        } else {
            0 // stalemate
        };
    }
    let ply_killers = ctx.killers[usize::from(ply)];
    order_moves(
        board,
        &mut moves,
        tt_move,
        ctx.order_noise,
        ply_killers,
        &ctx.history,
    );

    let alpha_orig = alpha;
    let mut best_score = -INFINITY;
    let mut best_move: Option<Move> = None;
    for (move_index, mv) in moves.into_iter().enumerate() {
        let quiet = !is_tactical(board, mv);
        board.commit_verified_move(mv);
        let mut child_pv = Vec::new();

        // Principal variation search: only the first move gets the full
        // window; the rest are probed with a zero-width window and re-searched
        // only when they beat alpha (a rare event with good move ordering).
        let score = if move_index == 0 {
            -negamax(
                board,
                depth - 1,
                ply + 1,
                -beta,
                -alpha,
                true,
                ctx,
                &mut child_pv,
            )
        } else {
            // Late move reduction: late quiet moves that neither escape nor
            // give check (and aren't killers) are probed a ply shallower.
            let reduce = quiet
                && depth >= LMR_MIN_DEPTH
                && move_index >= LMR_MIN_MOVE_INDEX
                && !in_check
                && ply_killers[0] != Some(mv)
                && ply_killers[1] != Some(mv)
                && !board.in_check(board.turn);
            let probe_depth = if reduce { depth - 2 } else { depth - 1 };

            let mut score = -negamax(
                board,
                probe_depth,
                ply + 1,
                -alpha - 1,
                -alpha,
                true,
                ctx,
                &mut child_pv,
            );
            // the reduced probe failed high: verify at full depth
            if reduce && score > alpha && !ctx.aborted {
                child_pv.clear();
                score = -negamax(
                    board,
                    depth - 1,
                    ply + 1,
                    -alpha - 1,
                    -alpha,
                    true,
                    ctx,
                    &mut child_pv,
                );
            }
            // inside the window: re-search with the full window for an exact score
            if score > alpha && score < beta && !ctx.aborted {
                child_pv.clear();
                score = -negamax(
                    board,
                    depth - 1,
                    ply + 1,
                    -beta,
                    -alpha,
                    true,
                    ctx,
                    &mut child_pv,
                );
            }
            score
        };
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
            // quiet moves that refute the position feed the killer/history
            // ordering heuristics
            if quiet {
                ctx.record_quiet_cutoff(mv, ply, depth);
            }
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
/// that the static evaluation is only applied to quiet positions. Hopeless
/// captures are skipped by delta pruning and by a negative static exchange
/// evaluation ([`see`]).
///
/// When the side to move is in check, standing pat on the static evaluation
/// would be meaningless (the position is not quiet, and the eval knows nothing
/// about the attack), so *all* evasions are searched instead, and a position
/// with none is mate — `ply` keeps the mate score root-relative.
#[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
fn quiescence(
    board: &mut Board,
    ply: u8,
    mut alpha: i32,
    beta: i32,
    ctx: &mut SearchContext,
) -> i32 {
    if ctx.count_node_and_check_abort() {
        return 0;
    }

    // Transposition-table probe: an entry of any depth is at least as informed
    // as this quiescence node, so the usual bound checks can cut off.
    let tt_entry = ctx.tt.probe(board.zobrist_key);
    if let Some(entry) = tt_entry {
        let score = score_from_tt(entry.score, ply);
        match entry.bound {
            Bound::Exact => return score,
            Bound::Lower if score >= beta => return score,
            Bound::Upper if score <= alpha => return score,
            _ => {}
        }
    }
    let tt_move = tt_entry.and_then(|entry| entry.mv);

    let in_check = board.in_check(board.turn);
    let alpha_orig = alpha;
    let mut best_score = -INFINITY;
    let mut stand_pat = -INFINITY;

    if !in_check {
        stand_pat = evaluate(board);
        if stand_pat >= beta {
            return stand_pat;
        }
        best_score = stand_pat;
        if stand_pat > alpha {
            alpha = stand_pat;
        }
    }

    let mut moves = board.generate_moves(board.turn);
    if in_check {
        // every evasion is searched: the position stays tactical until the
        // check is resolved
        if moves.is_empty() {
            return -(MATE_SCORE - i32::from(ply));
        }
    } else {
        moves.retain(|&m| is_tactical(board, m));
    }
    order_moves(
        board,
        &mut moves,
        tt_move,
        ctx.order_noise,
        [None; 2],
        &ctx.history,
    );

    let mut best_move: Option<Move> = None;
    for mv in moves {
        if !in_check {
            // Delta pruning: when even winning the victim plus a safety margin
            // cannot lift alpha, the capture is hopeless — skip it. Promotions
            // are exempt (they gain a queen's worth of material on top of the
            // victim).
            let victim = match mv.get_special_move() {
                SpecialMove::NormalMove => {
                    Some(board.get_piece_type_containing_position(mv.get_dest()))
                }
                SpecialMove::EnPassant => Some(Piece::Pawn),
                SpecialMove::Promotion | SpecialMove::Castle => None,
            };
            if let Some(victim) = victim
                && stand_pat + piece_value(victim) + DELTA_MARGIN <= alpha
            {
                continue;
            }

            // SEE pruning: a capture that loses material against best defence
            // cannot rescue a position where standing pat already failed low
            if mv.get_special_move() == SpecialMove::NormalMove && see(board, mv) < 0 {
                continue;
            }
        }

        board.commit_verified_move(mv);
        let score = -quiescence(board, ply + 1, -beta, -alpha, ctx);
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
        }
        if alpha >= beta {
            break;
        }
    }

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
        0,
        bound,
    );

    best_score
}

/// Returns `true` if the side to move has any piece besides pawns and the
/// king. Null-move pruning is unsound without one: pawn/king endgames are
/// where zugzwang (every move loses) is common.
fn has_non_pawn_material(board: &Board) -> bool {
    (board.get_piece_bitboard(Piece::Knight, board.turn)
        | board.get_piece_bitboard(Piece::Bishop, board.turn)
        | board.get_piece_bitboard(Piece::Rook, board.turn)
        | board.get_piece_bitboard(Piece::Queen, board.turn))
    .is_not_empty()
}

/// Bitboard of every piece of *either* colour attacking `sq` under the given
/// `occupancy` (which may differ from the board's as an exchange strips
/// pieces, revealing x-ray attackers). Mirrors the projection trick of
/// [`Board::is_square_attacked_occ`], but collects the attackers instead of
/// testing for one.
fn attackers_to(board: &Board, sq: usize, occupancy: Bitboard) -> Bitboard {
    // a white pawn attacks sq from the squares a black pawn on sq would attack
    let pawns = (PAWN_ATTACKS[usize::from(BLACK)][sq]
        & board.get_piece_bitboard(Piece::Pawn, WHITE))
        | (PAWN_ATTACKS[usize::from(WHITE)][sq] & board.get_piece_bitboard(Piece::Pawn, BLACK));

    let knights = KNIGHT_MOVES[sq]
        & (board.get_piece_bitboard(Piece::Knight, WHITE)
            | board.get_piece_bitboard(Piece::Knight, BLACK));
    let kings = KING_RING_MOVES[sq]
        & (board.get_piece_bitboard(Piece::King, WHITE)
            | board.get_piece_bitboard(Piece::King, BLACK));

    let queens = board.get_piece_bitboard(Piece::Queen, WHITE)
        | board.get_piece_bitboard(Piece::Queen, BLACK);

    let bishop_entry = BISHOP_MAGICS[sq];
    let bishop_attacks = BISHOP_ATTACKS
        [bishop_entry.magic_index(occupancy & BISHOP_BLOCKERS[sq]) + bishop_entry.offset];
    let diagonal = bishop_attacks
        & (board.get_piece_bitboard(Piece::Bishop, WHITE)
            | board.get_piece_bitboard(Piece::Bishop, BLACK)
            | queens);

    let rook_entry = ROOK_MAGICS[sq];
    let rook_attacks =
        ROOK_ATTACKS[rook_entry.magic_index(occupancy & ROOK_BLOCKERS[sq]) + rook_entry.offset];
    let straight = rook_attacks
        & (board.get_piece_bitboard(Piece::Rook, WHITE)
            | board.get_piece_bitboard(Piece::Rook, BLACK)
            | queens);

    pawns | knights | kings | diagonal | straight
}

/// The least valuable piece of `side` within `attackers`, as
/// `(piece, square)`, or `None` if `side` has no attacker in the set.
fn least_valuable_attacker(
    board: &Board,
    attackers: Bitboard,
    side: Turn,
) -> Option<(Piece, usize)> {
    for piece in [
        Piece::Pawn,
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::King,
    ] {
        let candidates = attackers & board.get_piece_bitboard(piece, side);
        if candidates.is_not_empty() {
            return Some((piece, candidates.trailing_zeros()));
        }
    }
    None
}

/// Static exchange evaluation: the material balance (in [`piece_value`]s, from
/// the mover's perspective) of the capture `mv` after the best possible
/// sequence of recaptures on the destination square by both sides. Standard
/// iterative swap algorithm; each side always recaptures with its least
/// valuable attacker, x-rays revealed as pieces come off, and negamax backup
/// lets either side stop the exchange early.
///
/// Only meaningful for normal captures (no promotion / en-passant geometry).
fn see(board: &Board, mv: Move) -> i32 {
    let (origin, dest) = mv.get_org_and_dest();
    let sq = dest.as_usize();

    // gain[d]: speculative material balance for the side moving at depth d
    let mut gain = [0_i32; 33];
    let mut d = 0;
    gain[0] = piece_value(board.get_piece_type_containing_position(dest));

    let mut occupancy = !board.empty_tiles;
    let mut attacker = board.get_piece_type_containing_position(origin);
    let mut from_sq = origin.as_usize();
    let mut side = board.turn;

    while d + 1 < gain.len() {
        d += 1;
        gain[d] = piece_value(attacker) - gain[d - 1];
        // both capturing and standing pat lose for the side to move: the
        // remainder of the exchange cannot change the outcome
        if gain[d].max(-gain[d - 1]) < 0 {
            break;
        }
        occupancy.clear_square(from_sq);
        side = !side;

        let attackers = attackers_to(board, sq, occupancy) & occupancy;
        let Some((piece, next_from)) = least_valuable_attacker(
            board,
            attackers & board.player_boards[usize::from(side)],
            side,
        ) else {
            break;
        };
        // a king can only join the exchange if the square is otherwise
        // undefended (capturing into a defended square would be illegal)
        if piece == Piece::King
            && (attackers & board.player_boards[usize::from(!side)]).is_not_empty()
        {
            break;
        }
        attacker = piece;
        from_sq = next_from;
    }

    // negamax the speculative gains back to the root of the exchange; the
    // deepest gain[d] belongs to a capture that never happened and is dropped
    while d > 1 {
        d -= 1;
        gain[d - 1] = -((-gain[d - 1]).max(gain[d]));
    }
    gain[0]
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
/// Ordering score of a ply's first killer move: below every capture, above all
/// other quiet moves.
const KILLER_0_SCORE: i32 = 4_000;
/// Ordering score of a ply's second killer move.
const KILLER_1_SCORE: i32 = 3_900;
/// Cap on a history entry, chosen so that a history-boosted quiet move (plus
/// the Lazy SMP jitter) can never outrank a killer.
const MAX_HISTORY_SCORE: i32 = 3_000;
/// Safety margin for delta pruning in the quiescence search: a capture is
/// skipped when winning the victim plus this margin still cannot lift alpha.
const DELTA_MARGIN: i32 = 200;

/// Order moves so the most forcing ones are searched first: the
/// transposition-table move, then promotions and captures (MVV-LVA: most
/// valuable victim, least valuable attacker), then the ply's killer moves,
/// then the remaining quiet moves by history score. A non-zero `noise` seed
/// perturbs the order of equal-ranked (quiet) moves so Lazy SMP helpers
/// diverge.
fn order_moves(
    board: &Board,
    moves: &mut [Move],
    tt_move: Option<Move>,
    noise: u64,
    killers: [Option<Move>; 2],
    history: &[[i32; 64]; 64],
) {
    let mut scored: Vec<(i32, Move)> = moves
        .iter()
        .map(|&mv| {
            (
                move_order_score(board, mv, tt_move, killers, history) + order_jitter(noise, mv),
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
/// above attacker value), then the killer moves, then the remaining quiet
/// moves by their history score.
fn move_order_score(
    board: &Board,
    mv: Move,
    tt_move: Option<Move>,
    killers: [Option<Move>; 2],
    history: &[[i32; 64]; 64],
) -> i32 {
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
    } else if score == 0 {
        // a quiet move (no capture, no promotion): killers first, then history
        if killers[0] == Some(mv) {
            return KILLER_0_SCORE;
        }
        if killers[1] == Some(mv) {
            return KILLER_1_SCORE;
        }
        let (origin, dest) = mv.get_org_and_dest();
        score = history[origin.as_usize()][dest.as_usize()];
    }
    score
}

#[cfg(test)]
mod tests {
    use super::see;
    use crate::chess_engine::board::Board;
    use crate::chess_engine::utils::init_tables;

    /// SEE of the legal move `mv` (UCI notation) in `fen`.
    fn see_of(fen: &str, mv: &str) -> i32 {
        init_tables();
        let mut board = Board::from_fen(fen).unwrap();
        let mv = board
            .generate_moves(board.turn)
            .into_iter()
            .find(|m| m.to_string() == mv)
            .expect("move should be legal in the test position");
        see(&board, mv)
    }

    #[test]
    fn see_capturing_an_undefended_piece_wins_its_value() {
        // rook takes a queen nobody defends
        assert_eq!(see_of("k7/8/8/3q4/8/8/3R4/K7 w - - 0 1", "d2d5"), 900);
    }

    #[test]
    fn see_pawn_takes_defended_queen_still_wins() {
        // d4 pawn takes the c5 queen; the d6 pawn recaptures: 900 - 100 > 0
        assert!(see_of("k7/8/3p4/2q5/3P4/8/8/K7 w - - 0 1", "d4c5") > 0);
    }

    #[test]
    fn see_rook_takes_defended_pawn_loses() {
        // rook grabs the d5 pawn but the c6 pawn recaptures: 100 - 500 < 0
        assert!(see_of("k7/8/2p5/3p4/8/8/3R4/K7 w - - 0 1", "d2d5") < 0);
    }

    #[test]
    fn see_xray_recapture_is_seen() {
        // RxR on d5 looks equal, but white's second rook on d1 backs the
        // exchange up through the first: win a whole rook
        assert!(see_of("k7/8/8/3r4/8/8/3R4/K2R4 w - - 0 1", "d2d5") >= 500);
    }
}
