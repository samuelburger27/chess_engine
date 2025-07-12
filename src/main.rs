use std::io;

mod fen_parser;
mod board;
mod move_generation;
mod uci;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = String::new();
    io::stdin()
        .read_line(&mut buffer)
        .expect("Failed to read input");

    if buffer.trim() == "uci" {
        crate::uci::uci_protocol()?;
        return Ok(());
    }
    println!("Currently only UCI protocol is supported, stopping");
    return Ok(());
}
