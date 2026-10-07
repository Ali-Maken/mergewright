//! A backend whose full-screen clear erases row by row.
//!
//! ratatui clears the whole screen (`ESC[2J`) when the window changes size.
//! Ghostty frees Kitty image *data* on that clear (ADR-0012), which made every
//! resize retransmit the whole viewport: 32–41 ms against a 16 ms budget.
//! Erasing each row with `ESC[2K` leaves the same blank screen and keeps the
//! uploads — checked in a real Ghostty 1.3.1 by `examples/kitty_check.rs`
//! (m1-the-graph/25): row erase then placements only, 0 errors; full clear
//! then placements only, one `ENOENT` per strip.

use std::io::{self, Write};

use ratatui::backend::{Backend, ClearType, WindowSize};
use ratatui::buffer::Cell;
use ratatui::layout::{Position, Size};

/// `backend`, with `ClearType::All` done as one `ESC[2K` per row.
#[derive(Debug)]
pub struct RowClear<B> {
    inner: B,
    /// Set by [`RowClear::cursor_given_up`]: showing the cursor already
    /// failed once and was handled, so a later ask is answered `Ok`.
    cursor_given_up: bool,
}

impl<B> RowClear<B> {
    /// Wrap `inner`.
    pub const fn new(inner: B) -> Self {
        Self {
            inner,
            cursor_given_up: false,
        }
    }

    /// The caller tried to show the cursor, it failed, and the caller has
    /// handled that (`startup::put_away`). ratatui's `Terminal` asks once
    /// more as it drops and `eprintln!`s a failure, and `eprintln!` panics
    /// when stderr is the same dead terminal (m8/13g: the hang-up's exit
    /// 101, ratatui-core 0.1.2 `terminal.rs:483`). From here on the ask is
    /// answered `Ok` without writing: the terminal it would write to has
    /// already refused.
    pub fn cursor_given_up(&mut self) {
        self.cursor_given_up = true;
    }
}

impl<B> Write for RowClear<B>
where
    B: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<B> Backend for RowClear<B>
where
    B: Backend<Error = io::Error> + Write,
{
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.inner.draw(content)
    }

    fn append_lines(&mut self, n: u16) -> io::Result<()> {
        self.inner.append_lines(n)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        if self.cursor_given_up {
            // Already failed and handled by the caller; see
            // `RowClear::cursor_given_up`.
            return Ok(());
        }
        self.inner.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.inner.get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.inner.set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.clear_region(ClearType::All)
    }

    /// The screen erased a row at a time, cursor saved and restored around
    /// it; every other kind of clear goes through unchanged.
    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        if clear_type != ClearType::All {
            return self.inner.clear_region(clear_type);
        }
        let rows = self.inner.size()?.height;
        let mut bytes = Vec::with_capacity(usize::from(rows) * 12 + 4);
        bytes.extend_from_slice(b"\x1b7");
        for row in 1..=rows {
            // `write!` into a `Vec` cannot fail.
            write!(bytes, "\x1b[{row};1H\x1b[2K").ok();
        }
        bytes.extend_from_slice(b"\x1b8");
        self.inner.write_all(&bytes)?;
        Write::flush(&mut self.inner)
    }

    fn size(&self) -> io::Result<Size> {
        self.inner.size()
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.inner.window_size()
    }

    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A three-row screen that records its bytes and the clears it was asked
    /// for.
    #[derive(Default)]
    struct Screen {
        bytes: Vec<u8>,
        passed: Vec<ClearType>,
    }

    impl Write for Screen {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Backend for Screen {
        type Error = io::Error;

        fn draw<'a, I>(&mut self, _: I) -> io::Result<()>
        where
            I: Iterator<Item = (u16, u16, &'a Cell)>,
        {
            Ok(())
        }

        fn hide_cursor(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn show_cursor(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn get_cursor_position(&mut self) -> io::Result<Position> {
            Ok(Position::ORIGIN)
        }

        fn set_cursor_position<P: Into<Position>>(&mut self, _: P) -> io::Result<()> {
            Ok(())
        }

        fn clear(&mut self) -> io::Result<()> {
            self.clear_region(ClearType::All)
        }

        fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
            self.passed.push(clear_type);
            Ok(())
        }

        fn size(&self) -> io::Result<Size> {
            Ok(Size::new(10, 3))
        }

        fn window_size(&mut self) -> io::Result<WindowSize> {
            Ok(WindowSize {
                columns_rows: Size::new(10, 3),
                pixels: Size::new(80, 48),
            })
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_full_clear_erases_each_row_and_never_writes_esc_2j() {
        let mut backend = RowClear::new(Screen::default());
        backend.clear().unwrap();
        let written = String::from_utf8(backend.inner.bytes).unwrap();
        assert_eq!(
            written,
            "\x1b7\x1b[1;1H\x1b[2K\x1b[2;1H\x1b[2K\x1b[3;1H\x1b[2K\x1b8"
        );
        assert!(backend.inner.passed.is_empty());
    }

    /// A screen whose cursor can no longer be shown: the terminal has
    /// hung up.
    struct Gone;

    impl Write for Gone {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from_raw_os_error(5))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from_raw_os_error(5))
        }
    }

    #[test]
    fn a_cursor_given_up_is_not_asked_for_again() {
        let mut backend = RowClear::new(ratatui::backend::CrosstermBackend::new(Gone));
        assert!(backend.show_cursor().is_err(), "the dead terminal refuses");
        backend.cursor_given_up();
        assert!(backend.show_cursor().is_ok(), "the retry is answered");
    }

    #[test]
    fn other_clears_go_through() {
        let mut backend = RowClear::new(Screen::default());
        backend.clear_region(ClearType::CurrentLine).unwrap();
        assert_eq!(backend.inner.passed, vec![ClearType::CurrentLine]);
        assert!(backend.inner.bytes.is_empty());
    }
}
