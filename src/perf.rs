use crate::board_representation::board::Board;


pub fn perft(board: &Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let mut nodes = 0;
    let moves = board.generate_moves(board.turn);

    for mv in moves {
        let mut new_board = board.clone();
        new_board.commit_verified_move(&mv);
        nodes += perft(&new_board, depth - 1);
    }

    nodes
}

pub fn perft_divide(board: &Board, depth: u32) {
    let moves = board.generate_moves(board.turn);
    let mut total_nodes = 0;

    for mv in moves {
        let mut new_board = board.clone();
        new_board.commit_verified_move(&mv);
        let nodes = perft(&new_board, depth - 1);
        println!("{}: {}", mv.to_string(), nodes);
        total_nodes += nodes;
    }

    println!("Total nodes at depth {}: {}", depth, total_nodes);
}
