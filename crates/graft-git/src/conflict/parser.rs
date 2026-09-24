//! [`parse`]: git's markers read line by line into [`ConflictFile`]'s
//! regions, on the rules the parent module's doc sets out.

use super::{
    Base, Block, ConflictFile, Line, MarkerEnds, ParseError, ParseErrorKind, Region, Terminator,
    blob_id,
};

/// What one line is to the parser. A label is `None` for the bare marker
/// form (`<<<<<<<` with nothing after it).
#[derive(Clone, Copy)]
enum Marker<'a> {
    Start(Option<&'a [u8]>),
    Base(Option<&'a [u8]>),
    Separator,
    End(Option<&'a [u8]>),
}

/// How a line reads before the parser's state is consulted.
#[derive(Clone, Copy)]
enum LineKind<'a> {
    /// Not a marker.
    Plain,
    /// A marker of git's default size.
    Marker(Marker<'a>),
    /// A conflict opened with markers longer than seven characters.
    Oversize,
}

/// The marker length this parser reads: git's default `conflict-marker-size`.
const MARKER_SIZE: usize = 7;

impl<'a> Marker<'a> {
    /// Classifies a line's content (terminator already split off).
    fn of(content: &'a [u8]) -> LineKind<'a> {
        if content == b"=======" {
            return LineKind::Marker(Marker::Separator);
        }
        let build: fn(Option<&'a [u8]>) -> Marker<'a> = match content.first() {
            Some(b'<') => Marker::Start,
            Some(b'|') => Marker::Base,
            Some(b'>') => Marker::End,
            _ => return LineKind::Plain,
        };
        let head = content[0];
        let run = content.iter().take_while(|&&byte| byte == head).count();
        // Either a space and a label, or nothing: `<<<<<<< HEAD`, `<<<<<<<`.
        let label = match &content[run..] {
            [] => None,
            [b' ', label @ ..] => Some(label),
            _ => return LineKind::Plain,
        };
        if run == MARKER_SIZE {
            return LineKind::Marker(build(label));
        }
        // Only the opener is checked for an oversize run, and only outside a
        // block (see `Parser::outside`): see the module doc for why `=`, `|`
        // and `>` runs stay content.
        if run > MARKER_SIZE && head == b'<' {
            return LineKind::Oversize;
        }
        LineKind::Plain
    }
}

/// Which part of a block the parser is inside.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Ours,
    Base,
    Theirs,
}

/// A block being read.
struct Open {
    block: Block,
    section: Section,
    /// The 1-based line of its `<<<<<<<`, for `Unterminated`.
    started_at: usize,
}

/// The parser's state between lines.
#[derive(Default)]
struct Parser {
    regions: Vec<Region>,
    /// Plain lines not yet closed into a region.
    plain: Vec<Line>,
    open: Option<Open>,
}

impl Parser {
    /// Reads one raw line (terminator included) at 1-based `number`.
    fn line(&mut self, number: usize, raw: &[u8]) -> Result<(), ParseError> {
        let (content, end) = Terminator::split(raw);
        let kind = Marker::of(content);
        let stray = ParseError {
            line: number,
            what: ParseErrorKind::StrayMarker,
        };
        let Some(current) = self.open.as_mut() else {
            return self.outside(number, raw, kind, end, stray);
        };
        // Inside a block an oversize `<` run is content: a file this parser
        // got inside of has seven-character markers, so a longer run is a
        // line of a side, not a marker.
        let marker = match kind {
            LineKind::Marker(marker) => Some(marker),
            LineKind::Plain | LineKind::Oversize => None,
        };
        match (current.section, marker) {
            (_, Some(Marker::Start(_))) | (Section::Ours | Section::Base, Some(Marker::End(_))) => {
                Err(stray)
            }
            (Section::Ours, Some(Marker::Base(label))) => {
                current.block.base = Some(Base {
                    label: label.map(<[u8]>::to_vec),
                    lines: Vec::new(),
                });
                current.block.ends.base = end;
                current.section = Section::Base;
                Ok(())
            }
            (Section::Ours | Section::Base, Some(Marker::Separator)) => {
                current.block.ends.separator = end;
                current.section = Section::Theirs;
                Ok(())
            }
            (Section::Theirs, Some(Marker::End(label))) => {
                let Some(mut done) = self.open.take() else {
                    unreachable!("a block is open on this branch")
                };
                done.block.theirs_label = label.map(<[u8]>::to_vec);
                done.block.ends.end = end;
                self.regions.push(Region::Conflict(done.block));
                Ok(())
            }
            // A second `|||||||` or `=======`, or a `|||||||` after the
            // separator, is content: git writes one of each and never
            // escapes what it copies.
            (Section::Ours, None) => {
                current.block.ours.push(Line(raw.to_vec()));
                Ok(())
            }
            (Section::Base, None | Some(Marker::Base(_))) => {
                if let Some(base) = current.block.base.as_mut() {
                    base.lines.push(Line(raw.to_vec()));
                }
                Ok(())
            }
            (Section::Theirs, None | Some(Marker::Base(_) | Marker::Separator)) => {
                current.block.theirs.push(Line(raw.to_vec()));
                Ok(())
            }
        }
    }

    /// A line with no block open: plain text, or the start of one.
    fn outside(
        &mut self,
        number: usize,
        raw: &[u8],
        kind: LineKind<'_>,
        end: Terminator,
        stray: ParseError,
    ) -> Result<(), ParseError> {
        match kind {
            LineKind::Marker(Marker::Start(label)) => {
                if !self.plain.is_empty() {
                    self.regions
                        .push(Region::Plain(std::mem::take(&mut self.plain)));
                }
                self.open = Some(Open {
                    block: Block {
                        ours_label: label.map(<[u8]>::to_vec),
                        ours: Vec::new(),
                        base: None,
                        theirs_label: None,
                        theirs: Vec::new(),
                        ends: MarkerEnds {
                            start: end,
                            base: Terminator::None,
                            separator: Terminator::None,
                            end: Terminator::None,
                        },
                        reproduced: None,
                    },
                    section: Section::Ours,
                    started_at: number,
                });
                Ok(())
            }
            LineKind::Marker(_) => Err(stray),
            LineKind::Oversize => Err(ParseError {
                line: number,
                what: ParseErrorKind::MarkerSize,
            }),
            LineKind::Plain => {
                self.plain.push(Line(raw.to_vec()));
                Ok(())
            }
        }
    }

    /// The end of the input.
    fn finish(mut self) -> Result<Vec<Region>, ParseError> {
        if let Some(unfinished) = self.open {
            return Err(ParseError {
                line: unfinished.started_at,
                what: ParseErrorKind::Unterminated,
            });
        }
        if !self.plain.is_empty() {
            self.regions.push(Region::Plain(self.plain));
        }
        Ok(self.regions)
    }
}

/// Parses a working-tree file with git's conflict markers in it.
///
/// Accepts a file with no markers at all (one `Plain` region, or none for an
/// empty file). `render` gives back exactly `bytes` for every input this
/// accepts.
pub fn parse(bytes: &[u8]) -> Result<ConflictFile, ParseError> {
    let fingerprint = blob_id(bytes);
    let mut parser = Parser::default();
    for (index, raw) in bytes.split_inclusive(|&b| b == b'\n').enumerate() {
        parser.line(index + 1, raw)?;
    }
    Ok(ConflictFile {
        regions: parser.finish()?,
        fingerprint,
    })
}
