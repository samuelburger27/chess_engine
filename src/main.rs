use chess_engine::{board_representation::{board::Board, utils::init_tables}, perf::perft_divide};
pub use chess_engine::{
    board_representation::{board, magic_tables::find_magics},
    uci::uci_protocol,
};

use std::{io, sync::LazyLock};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tables();
    //let seed = 1234u64;
    //find_magics(Some(seed));

    // let mut board =
    //     Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")?;
    // perft_divide(&board, 5);
    // start pos
    let mut board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")?;
    board.print_board();
    println!("Testing using perf");
    perft_divide(&board, 1);
    //should be 193690690
    // let mut buffer = String::new();
    // io::stdin()
    //     .read_line(&mut buffer)
    //     .expect("Failed to read input");

    // if buffer.trim() == "uci" {
    //     uci_protocol()?;
    //     return Ok(());
    // }
    // println!("Currently only UCI protocol is supported, stopping");
    return Ok(());
}
