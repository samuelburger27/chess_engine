pub use chess_engine::{
    board::{Board, Position},
    fen_parser::{parse_fen, starting_pos_fen},
    move_generation::get_moves,
    perf::perft_divide,
    uci::uci_protocol,
};

use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // println!("Testing using perf");
    // let mut board =
    //     parse_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")?;
    // perft_divide(&board, 5);
    // should be 193690690
    let mut buffer = String::new();
    io::stdin()
        .read_line(&mut buffer)
        .expect("Failed to read input");

    if buffer.trim() == "uci" {
        uci_protocol()?;
        return Ok(());
    }
    println!("Currently only UCI protocol is supported, stopping");
    return Ok(());
}
