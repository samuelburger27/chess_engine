
use crate::board::Board;


pub fn perft(board: &Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let mut nodes = 0;
    let moves = board.get_all_moves(None);

    for mv in moves {
        let mut new_board = *board;
        new_board.commit_verified_move(&mv);
        nodes += perft(&new_board, depth - 1);
    }

    nodes
}

pub fn perft_divide(board: &Board, depth: u32) {
    let moves = board.get_all_moves(None);
    let mut total_nodes = 0;

    for mv in moves {
        let mut new_board = *board;
        new_board.commit_verified_move(&mv);
        let nodes = perft(&new_board, depth - 1);
        println!("{}: {}", mv.to_string(), nodes);
        total_nodes += nodes;
    }

    println!("Total nodes at depth {}: {}", depth, total_nodes);
}
