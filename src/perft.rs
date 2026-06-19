//! [Perft] (performance test): counting the leaf nodes of the move tree to a
//! fixed depth.
//!
//! Perft is the correctness oracle for move generation — if the engine counts
//! the same number of positions as a known-good reference, its generation,
//! make, and unmake logic are almost certainly correct. The integration tests
//! in `tests/perft_tests.rs` pin several positions to their published counts.
//!
//! [Perft]: https://www.chessprogramming.org/Perft

use crate::chess_engine::board::Board;

/// Counts the number of leaf nodes reachable from `board` in exactly `depth`
/// plies (a depth of `0` counts the position itself as one node).
///
/// ```
/// use chess_engine::chess_engine::board::Board;
/// use chess_engine::chess_engine::utils::init_tables;
/// use chess_engine::perft::perft;
///
/// init_tables();
/// let mut board = Board::new_start_pos().unwrap();
/// assert_eq!(perft(&mut board, 1), 20);
/// assert_eq!(perft(&mut board, 2), 400);
/// assert_eq!(perft(&mut board, 3), 8902);
/// ```
pub fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let mut nodes = 0;
    let moves = board.generate_moves(board.turn);

    for mv in moves {
        board.commit_verified_move(mv);
        nodes += perft(board, depth - 1);
        board.unmake_move();
    }

    nodes
}

/// Per-move perft breakdown: prints each root move with its node count, then
/// the total.
///
/// Used to localise move-generation bugs by diffing against another engine
/// (the UCI `go perft <n>` command calls this).
pub fn perft_divide(board: &mut Board, depth: u32) {
    let moves = board.generate_moves(board.turn);
    let mut total_nodes = 0;

    for mv in moves {
        board.commit_verified_move(mv);
        let nodes = perft(board, depth - 1);
        board.unmake_move();
        println!("{mv}: {nodes}");
        total_nodes += nodes;
    }

    println!("Total nodes at depth {depth}: {total_nodes}");
}
