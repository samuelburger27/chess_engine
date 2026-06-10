use crate::{
    chess_engine::{board::Board, engine::search::find_best_move},
    perft::perft_divide,
};

pub fn uci_protocol() -> Result<(), Box<dyn std::error::Error>> {
    println!("id name RustChessEngine");
    println!("id author Samuel Burger");
    println!("uciok");

    let mut input = String::new();
    let is_ready = true;
    let mut board: Board = Board::new_start_pos()?;

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
            "position" => {
                board = parse_position(&parts)?;
            }
            "go" => {
                parse_go(&parts, &mut board);
            }
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
        _ => {
            //println!("Starting search with dept 5");
            let search_res = find_best_move(board, 5);
            println!("info depth 1 score cp 0");
            if let Some(best_move) = search_res.best_move {
                println!("bestmove {}", best_move.to_string());
            }
        }
    }
    return true;
}

pub fn parse_position(parts: &[&str]) -> Result<Board, String> {
    let mut board = match parts[1] {
        "fen" => {
            if parts.len() < 7 {
                Err("FEN string is too short".to_string())
            } else {
                Board::from_fen(&parts[2..8].join(" "))
            }
        }
        "startpos" => Board::new_start_pos(),
        _ => Err("Not a valid position".to_string()),
    }?;

    if parts.len() > 8 && parts[8] == "moves" {
        for str_move in parts.iter().skip(9) {
            if !board.play_string_move(&str_move) {
                return Err(format!("Couldn't play move: {}", str_move));
            }
        }
    }
    return Ok(board);
}
