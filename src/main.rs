use chess_engine::{board_representation::board::Board, perft::perft_divide};
pub use chess_engine::{
    board_representation::{board, magic_tables::find_magics, utils::init_tables},
    uci::uci_protocol,
};

use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tables();
    // let seed = 1234u64;
    // find_magics(Some(seed));
    // REMOVE
    let mut board = Board::new_start_pos()?;
    //board.print_board();
    perft_divide(&mut board, 5);

    ///////////
    // let mut buffer = String::new();
    // io::stdin()
    //     .read_line(&mut buffer)
    //     .expect("Failed to read input");

    // if buffer.trim() == "uci" {
    //     uci_protocol()?;
    //     return Ok(());
    // }
    println!("Currently only UCI protocol is supported, stopping");
    Ok(())
}
