use chess_engine::{chess_engine::utils::init_tables, uci::uci_protocol};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tables();
    // the command loop replies to `uci` with the id/uciok handshake and
    // handles everything else (position, go, stop, quit, ...)
    uci_protocol()
}
