//! Binary entry point: initialise the lookup tables, then hand control to the
//! UCI command loop. See the [`chess_engine`] library crate for the engine
//! itself.

use chess_engine::{chess_engine::utils::init_tables, uci::uci_protocol};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tables();
    // the command loop replies to `uci` with the id/uciok handshake and
    // handles everything else (position, go, stop, quit, ...)
    uci_protocol()
}
