//! The Graft binary. Deliberately thin: everything it runs lives in the
//! `graft` library (`lib.rs`) so the same code can be driven from tests and
//! from the `terminal_probe` example. The event loop that ties the terminal
//! session, the panic hook and `update` together arrives in ticket 09.

fn main() {}
