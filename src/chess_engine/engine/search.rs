use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::chess_engine::engine::evaluation::evaluate;
use crate::chess_engine::piece::Piece;

use super::super::{board::Board, r#move::Move, r#move::SpecialMove};

pub const MATE_SCORE: i32 = 30_000;
/// Scores above this magnitude encode a forced mate.
pub const MATE_THRESHOLD: i32 = MATE_SCORE - 1_000;
const INFINITY: i32 = i32::MAX - 1;

/// How often (in nodes) the abort conditions are polled.
const ABORT_CHECK_INTERVAL: u64 = 2048;

pub const MAX_DEPTH: u8 = 64;

#[derive(Debug, Clone, Copy)]
pub struct SearchLimits {
    /// Maximum iterative-deepening depth.
    pub depth: u8,
    /// Hard wall-clock limit for the whole search, if any.
    pub deadline: Option<Instant>,
}

impl SearchLimits {
    pub fn depth(depth: u8) -> Self {
        SearchLimits {
            depth: depth.min(MAX_DEPTH),
            deadline: None,
        }
    }

    pub fn infinite() -> Self {
        SearchLimits {
            depth: MAX_DEPTH,
            deadline: None,
        }
    }
}

#[derive(Debug)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub depth: u8,
    pub nodes: u64,
    pub pv: Vec<Move>,
}

struct SearchContext<'a> {
    stop: &'a AtomicBool,
    deadline: Option<Instant>,
    nodes: u64,
    aborted: bool,
}

impl SearchContext<'_> {
    fn count_node_and_check_abort(&mut self) -> bool {
        self.nodes += 1;
        if self.nodes % ABORT_CHECK_INTERVAL == 0 {
            if self.stop.load(Ordering::Relaxed) {
                self.aborted = true;
            } else if let Some(deadline) = self.deadline {
                if Instant::now() >= deadline {
                    self.aborted = true;
                }
            }
        }
        self.aborted
    }
}

/// Backwards-compatible fixed-depth entry point.
pub fn find_best_move(board: &Board, depth: u8) -> SearchResult {
    let stop = AtomicBool::new(false);
    search_position(&mut board.clone(), SearchLimits::depth(depth), &stop, false)
}

/// Iterative-deepening driver. Searches `board` until the depth limit,
/// the deadline, or the stop flag ends the search, and returns the result of
/// the last fully completed iteration. When `report` is set, a UCI `info`
/// line is printed after every completed depth.
pub fn search_position(
    board: &mut Board,
    limits: SearchLimits,
    stop: &AtomicBool,
    report: bool,
) -> SearchResult {
    let start = Instant::now();
    let mut ctx = SearchContext {
        stop,
        deadline: limits.deadline,
        nodes: 0,
        aborted: false,
    };

    let mut result = SearchResult {
        best_move: None,
        score: 0,
        depth: 0,
        nodes: 0,
        pv: Vec::new(),
    };

    for depth in 1..=limits.depth.max(1) {
        let mut pv = Vec::new();
        let score = negamax(board, depth, 0, -INFINITY, INFINITY, &mut ctx, &mut pv);
        if ctx.aborted {
            break;
        }

        result = SearchResult {
            best_move: pv.first().copied(),
            score,
            depth,
            nodes: ctx.nodes,
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

    result.nodes = ctx.nodes;
    result
}

fn print_info(result: &SearchResult, start: Instant) {
    let elapsed = start.elapsed();
    let millis = elapsed.as_millis().max(1);
    let nps = (result.nodes as u128 * 1000) / millis;

    let score = if result.score.abs() >= MATE_THRESHOLD {
        // moves (not plies) until mate, negative when we are getting mated
        let plies = MATE_SCORE - result.score.abs();
        let moves = (plies + 1) / 2;
        format!("mate {}", if result.score > 0 { moves } else { -moves })
    } else {
        format!("cp {}", result.score)
    };

    let pv: Vec<String> = result.pv.iter().map(|m| m.to_string()).collect();
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

    let mut moves = board.generate_moves(board.turn);
    if moves.is_empty() {
        return if board.in_check(board.turn) {
            // mated: worse the closer to the root it happens
            -(MATE_SCORE - ply as i32)
        } else {
            0 // stalemate
        };
    }
    order_moves(board, &mut moves);

    let mut best_score = -INFINITY;
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

    best_score
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
        .filter(|m| is_tactical(board, m))
        .collect();
    order_moves(board, &mut moves);

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

fn is_tactical(board: &Board, mv: &Move) -> bool {
    match mv.get_special_move() {
        SpecialMove::Promotion | SpecialMove::EnPassant => true,
        SpecialMove::Castle => false,
        SpecialMove::NormalMove => !board.empty_tiles.is_square_set(mv.get_dest().into()),
    }
}

fn piece_value(piece: Piece) -> i32 {
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

/// Order moves so the most forcing ones are searched first: promotions and
/// captures (MVV-LVA: most valuable victim, least valuable attacker) before
/// quiet moves.
fn order_moves(board: &Board, moves: &mut [Move]) {
    let mut scored: Vec<(i32, Move)> = moves
        .iter()
        .map(|&mv| (move_order_score(board, &mv), mv))
        .collect();
    scored.sort_by_key(|(score, _)| -*score);
    for (slot, (_, mv)) in moves.iter_mut().zip(scored) {
        *slot = mv;
    }
}

fn move_order_score(board: &Board, mv: &Move) -> i32 {
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
