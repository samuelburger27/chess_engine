use crate::chess_engine::{engine::evaluation::evaluate};

use super::super::{board::Board, r#move::Move};

// A simple struct to hold the result of the search.
#[derive(Debug)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
}

pub fn find_best_move(board: &Board, depth: u8) -> SearchResult {
    search(board.clone(), depth, -i32::MAX, i32::MAX)
}

/// Principal Variation Search (PVS) using Negamax with Alpha-Beta Pruning.
///
/// This function searches for the best move from the given board position.
/// It returns a `SearchResult` containing the best move found and its evaluation score.
///
/// # Arguments
///
/// * `board` - The current chess board state.
/// * `depth` - The remaining depth to search.
/// * `alpha` - The lower bound of the score window (best score for the maximizing player).
/// * `beta` - The upper bound of the score window (best score for the minimizing player).
///
/// # Returns
///
/// The evaluation of the position. A positive score favors the current player.
fn search(mut board: Board, depth: u8, mut alpha: i32, beta: i32) -> SearchResult {
    // 1. Base Case: If we've reached the desired depth, evaluate the position.
    if depth == 0 {
        return SearchResult {
            best_move: None,
            score: evaluate(&board),
        };
    }

    let legal_moves = board.generate_moves(board.turn);

    // 2. Base Case: Check for checkmate or stalemate.
    if legal_moves.is_empty() {
        if board.in_check(board.turn) {
            // Return a very low score for checkmate.
            // The score is adjusted by depth so the engine prefers faster mates.
            return SearchResult {
                best_move: None,
                score: -30000 - depth as i32,
            };
        }
        // It's a stalemate.
        return SearchResult {
            best_move: None,
            score: 0,
        };
    }

    let mut best_move = None;

    // 3. Recursive Step: Iterate through all legal moves.
    for mv in legal_moves {
        board.commit_verified_move(mv);
        // Recursively call search for the opponent.
        // The score is negated because what's good for the opponent is bad for us.
        // Alpha and beta are swapped and negated.
        let result = search(board.clone(), depth - 1, -beta, -alpha);
        let score = -result.score;
        board.unmake_move();

        // 4. Alpha-Beta Pruning Logic
        if score >= beta {
            // This move is too good; the opponent will avoid this line.
            // This is a "beta cutoff".
            return SearchResult {
                best_move: Some(mv),
                score: beta,
            }; // Fail-hard beta cutoff
        }

        if score > alpha {
            // This move is the best we've seen so far in this position.
            alpha = score;
            best_move = Some(mv);
        }
    }

    SearchResult {
        best_move,
        score: alpha,
    }
}
