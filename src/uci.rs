use crate::board::Board;
use crate::fen_parser::parse_fen;
use crate::fen_parser::starting_pos_fen;
use crate::move_generation::get_moves;

pub fn uci_protocol() -> Result<(), Box<dyn std::error::Error>> {
    println!("id name RustChessEngine");
    println!("id author Samuel Burger");
    println!("uciok");

    let mut input = String::new();
    let is_ready = true;
    let mut board: Board = parse_fen(&starting_pos_fen())?;

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
            "go" => start_search(&board),
            _ => println!("Invalid command"),
        }
        input.clear();
        std::io::stdin().read_line(&mut input)?;
    }

    return Ok(());
}

fn start_search(board: &Board) {
    // TODO
    for (pos, (_, color)) in board {
        if color == Some(board.white_turn) {
            let moves = get_moves(&board, pos);
            if moves.len() == 0 {
                continue;
            }
            let move_to_print = moves[0].to_string();
            println!("{}", move_to_print);
            return;
        }
    }
}

fn parse_position(parts: &Vec<&str>) -> Result<Board, String> {
    let mut board = match parts[1] {
        "fen" => parse_fen(parts[2]),
        "startpos" => parse_fen(&starting_pos_fen()),
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
