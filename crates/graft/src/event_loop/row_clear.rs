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
pub struct RowClear<B>(pub B);

impl<B> Write for RowClear<B>
where
    B: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
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
        self.0.draw(content)
    }

    fn append_lines(&mut self, n: u16) -> io::Result<()> {
        self.0.append_lines(n)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.0.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.0.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.0.get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.0.set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.clear_region(ClearType::All)
    }

    /// The screen erased a row at a time, cursor saved and restored around
    /// it; every other kind of clear goes through unchanged.
    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        if clear_type != ClearType::All {
            return self.0.clear_region(clear_type);
        }
        let rows = self.0.size()?.height;
        let mut bytes = Vec::with_capacity(usize::from(rows) * 12 + 4);
        bytes.extend_from_slice(b"\x1b7");
        for row in 1..=rows {
            // `write!` into a `Vec` cannot fail.
            write!(bytes, "\x1b[{row};1H\x1b[2K").ok();
        }
        bytes.extend_from_slice(b"\x1b8");
        self.0.write_all(&bytes)?;
        Write::flush(&mut self.0)
    }

    fn size(&self) -> io::Result<Size> {
        self.0.size()
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.0.window_size()
    }

    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.0)
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
        let mut backend = RowClear(Screen::default());
        backend.clear().unwrap();
        let written = String::from_utf8(backend.0.bytes).unwrap();
        assert_eq!(
            written,
            "\x1b7\x1b[1;1H\x1b[2K\x1b[2;1H\x1b[2K\x1b[3;1H\x1b[2K\x1b8"
        );
        assert!(backend.0.passed.is_empty());
    }

    #[test]
    fn other_clears_go_through() {
        let mut backend = RowClear(Screen::default());
        backend.clear_region(ClearType::CurrentLine).unwrap();
        assert_eq!(backend.0.passed, vec![ClearType::CurrentLine]);
        assert!(backend.0.bytes.is_empty());
    }
}
