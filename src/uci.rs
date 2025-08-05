use crate::{
    board_representation::board::Board,
    perft::{perft_divide},
};

pub fn uci_protocol() -> Result<(), Box<dyn std::error::Error>> {
    println!("id name RustChessEngine");
    println!("id author Samuel Burger");
    println!("uciok");

    let mut input = String::new();
    let is_ready = true;
    let mut board: Board = Board::new_empty();

    std::io::stdin().read_line(&mut input)?;
    while input != "quit\n" {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        if parts.len() == 0 {
            input.clear();
            std::io::stdin().read_line(&mut input)?;
            continue;
        }
        match parts[0] {
            "isready" => {
                if is_ready {
                    println!("readyok");
                }
            }
            "position" => board = parse_position(&parts)?,
            "go" => {parse_go(&parts, &mut board);},
            _ => println!("Invalid command"),
        }
        input.clear();
        std::io::stdin().read_line(&mut input)?;
    }

    return Ok(());
}

fn parse_go(parts: &[&str], board: &mut Board) -> bool {
    match parts[1] {
        "perf" => {
            if let Ok(depth) = parts[2].parse::<u32>() {
                perft_divide(board, depth);
            }
        }
        _ => println!("Only perf is currently supported"),
    }
    let moves = board.generate_moves(board.turn);
    if !moves.is_empty() {
        let move_to_print = moves[0].to_string();
        println!("{}", move_to_print);
    }
    return true;
}

fn parse_position(parts: &[&str]) -> Result<Board, String> {
    let mut board = match parts[1] {
        "fen" => Board::from_fen(parts[2]),
        "startpos" => Board::new_start_pos(),
        _ => Err("Not a valid position".to_string()),
    }?;

    if parts.len() > 4 && parts[3] == "moves" {
        for str_move in parts.iter().skip(4) {
            if !board.play_string_move(&str_move) {
                return Err(format!("Couldn't play move: {}", str_move));
            }
        }
    }
    return Ok(board);
}
