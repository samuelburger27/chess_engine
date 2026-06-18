//! The [Universal Chess Interface][uci] (UCI) protocol loop.
//!
//! [`uci_protocol`] reads commands from stdin and replies on stdout, holding the
//! current [`Board`] between commands. Supported commands: `uci`, `isready`,
//! `ucinewgame`, `position` (`startpos`/`fen`, with optional `moves`), `go`
//! (`depth`, `movetime`, `wtime`/`btime`/`winc`/`binc`, `infinite`, or
//! `perft N`), `stop`, `d` (print the board), and `quit`.
//!
//! Searches run on a background thread so `stop` can interrupt them; the search
//! result is reported as a `bestmove` line when it finishes. This module also
//! owns time allocation: it converts the clock into a per-move budget of
//! roughly `clock/25 + inc/2`, capped at half the clock, less a small
//! `MOVE_OVERHEAD_MS` safety margin.
//!
//! [uci]: https://www.chessprogramming.org/UCI

use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::{
    chess_engine::{
        board::{Board, WHITE},
        engine::search::{search_position, SearchLimits},
    },
    perft::perft_divide,
};

/// Engine name reported in the `uci` handshake.
const ENGINE_NAME: &str = "RustChessEngine";
/// Engine author reported in the `uci` handshake.
const ENGINE_AUTHOR: &str = "Samuel Burger";

/// Safety margin subtracted from the clock so the engine never flags
/// because of I/O latency.
const MOVE_OVERHEAD_MS: u64 = 30;

/// The mutable state the protocol loop carries between commands: the current
/// position, the shared stop flag, and the handle of any running search thread.
struct EngineState {
    board: Board,
    stop: Arc<AtomicBool>,
    search_thread: Option<JoinHandle<()>>,
}

impl EngineState {
    /// Signals any running search to stop, joins its thread, and resets the stop
    /// flag for the next search.
    fn stop_search(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.search_thread.take() {
            let _ = handle.join();
        }
        self.stop.store(false, Ordering::Relaxed);
    }
}

/// Runs the UCI command loop, reading from stdin until `quit` or end-of-input.
///
/// # Errors
///
/// Returns an error if reading a line from stdin fails or if the initial
/// start position cannot be built.
pub fn uci_protocol() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = EngineState {
        board: Board::new_start_pos()?,
        stop: Arc::new(AtomicBool::new(false)),
        search_thread: None,
    };

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "uci" => print_identity(),
            "isready" => println!("readyok"),
            "setoption" => (), // no options supported yet
            "ucinewgame" => {
                state.stop_search();
                state.board = Board::new_start_pos()?;
            }
            "position" => {
                state.stop_search();
                match parse_position(&parts) {
                    Ok(board) => state.board = board,
                    Err(err) => println!("info string error: {err}"),
                }
            }
            "go" => handle_go(&parts, &mut state),
            "stop" => state.stop_search(),
            "d" => state.board.print_board(),
            "quit" => break,
            // per the UCI spec unknown tokens are ignored
            _ => (),
        }
    }

    state.stop_search();
    Ok(())
}

/// Sends the `id name`/`id author`/`uciok` handshake in response to `uci`.
fn print_identity() {
    println!("id name {ENGINE_NAME}");
    println!("id author {ENGINE_AUTHOR}");
    println!("uciok");
}

/// Handles a `go` command: runs `perft` synchronously, or otherwise spawns a
/// background search thread that prints the chosen `bestmove` when it finishes.
fn handle_go(parts: &[&str], state: &mut EngineState) {
    state.stop_search();

    // perft is handled synchronously; it is a debugging command
    if let Some(index) = parts.iter().position(|&p| p == "perft" || p == "perf") {
        if let Some(depth) = parts.get(index + 1).and_then(|d| d.parse::<u32>().ok()) {
            perft_divide(&mut state.board, depth);
        } else {
            println!("info string error: go perft requires a depth");
        }
        return;
    }

    let limits = parse_go_limits(parts, &state.board);

    let mut board = state.board.clone();
    let stop = Arc::clone(&state.stop);
    state.search_thread = Some(std::thread::spawn(move || {
        let result = search_position(&mut board, limits, &stop, true);
        match result.best_move {
            Some(best_move) => println!("bestmove {}", best_move.to_string()),
            None => println!("bestmove 0000"),
        }
    }));
}

/// Turns the tokens of a `go` command into [`SearchLimits`], applying the time
/// allocation described in the [module docs](self). A bare `go` (no depth and no
/// clocks) falls back to a three-second budget so the engine stays responsive.
fn parse_go_limits(parts: &[&str], board: &Board) -> SearchLimits {
    let mut depth: Option<u8> = None;
    let mut movetime: Option<u64> = None;
    let mut wtime: Option<u64> = None;
    let mut btime: Option<u64> = None;
    let mut winc: Option<u64> = None;
    let mut binc: Option<u64> = None;

    let mut iter = parts.iter().skip(1);
    while let Some(&token) = iter.next() {
        match token {
            "infinite" => return SearchLimits::infinite(),
            "depth" | "movetime" | "wtime" | "btime" | "winc" | "binc" | "movestogo" => {
                let Some(value) = iter.next().and_then(|v| v.parse::<u64>().ok()) else {
                    continue;
                };
                match token {
                    "depth" => depth = Some(value.min(u64::from(u8::MAX)) as u8),
                    "movetime" => movetime = Some(value),
                    "wtime" => wtime = Some(value),
                    "btime" => btime = Some(value),
                    "winc" => winc = Some(value),
                    "binc" => binc = Some(value),
                    _ => (),
                }
            }
            _ => (),
        }
    }

    let budget_ms = if let Some(movetime) = movetime {
        Some(movetime.saturating_sub(MOVE_OVERHEAD_MS).max(1))
    } else {
        let (my_time, my_inc) = if board.turn == WHITE {
            (wtime, winc.unwrap_or(0))
        } else {
            (btime, binc.unwrap_or(0))
        };
        my_time.map(|remaining| {
            // simple allocation: a slice of the remaining clock plus half the
            // increment, never more than half the clock
            let budget = remaining / 25 + my_inc / 2;
            budget
                .min(remaining.saturating_sub(MOVE_OVERHEAD_MS) / 2)
                .max(1)
        })
    };

    let mut limits = match depth {
        Some(depth) => SearchLimits::depth(depth),
        None => SearchLimits::infinite(),
    };
    // bare `go` (no depth and no clocks) would search forever; give it a
    // small default budget so the engine stays usable interactively
    if budget_ms.is_none() && depth.is_none() {
        limits.deadline = Some(Instant::now() + Duration::from_secs(3));
        return limits;
    }
    limits.deadline = budget_ms.map(|ms| Instant::now() + Duration::from_millis(ms));
    limits
}

/// Parses a `position` command into a [`Board`].
///
/// Accepts `position startpos` or `position fen <fields>`, each optionally
/// followed by `moves <m1> <m2> ...` which are applied in order. A FEN missing
/// the half-move/full-move counters is tolerated (defaults `0`/`1` are filled
/// in).
///
/// # Errors
///
/// Returns `Err` if the command is malformed, the FEN is invalid, or one of the
/// listed moves is illegal in the resulting position.
pub fn parse_position(parts: &[&str]) -> Result<Board, String> {
    if parts.len() < 2 {
        return Err("position requires startpos or fen".to_string());
    }

    let moves_index = parts.iter().position(|&p| p == "moves");

    let mut board = match parts[1] {
        "startpos" => Board::new_start_pos()?,
        "fen" => {
            let fen_end = moves_index.unwrap_or(parts.len());
            let mut fen_fields: Vec<&str> = parts[2..fen_end].to_vec();
            // tolerate FENs without the halfmove/fullmove counters
            if fen_fields.len() == 4 {
                fen_fields.push("0");
            }
            if fen_fields.len() == 5 {
                fen_fields.push("1");
            }
            Board::from_fen(&fen_fields.join(" "))?
        }
        other => return Err(format!("invalid position type: {other}")),
    };

    if let Some(index) = moves_index {
        for str_move in &parts[index + 1..] {
            if !board.play_string_move(str_move) {
                return Err(format!("couldn't play move: {str_move}"));
            }
        }
    }
    Ok(board)
}
