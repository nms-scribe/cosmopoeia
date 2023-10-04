use std::io;
use serde_json::ser::Formatter;

// Mostly copy-paste from serde_json, but designed to output arrays inline at any nesting above one, for serializing namers in an array.

fn indent<W>(wr: &mut W, n: usize, s: &[u8]) -> io::Result<()>
where
    W: ?Sized + io::Write,
{
    for _ in 0..n {
        wr.write_all(s)?;
    }

    Ok(())
}

/// This structure pretty prints a JSON value to make it human readable.
#[derive(Clone, Debug)]
pub(crate) struct PrettyFormatter<'indent> {
    current_indent: usize,
    has_value: bool,
    array_nesting: usize,
    indent: &'indent [u8],
}

impl<'indent> PrettyFormatter<'indent> {
    /// Construct a pretty printer formatter that defaults to using two spaces for indentation.
    pub(crate) const fn new() -> Self {
        PrettyFormatter::with_indent(b"  ")
    }

    /// Construct a pretty printer formatter that uses the `indent` string for indentation.
    pub(crate) const fn with_indent(indent: &'indent [u8]) -> Self {
        PrettyFormatter {
            current_indent: 0,
            has_value: false,
            array_nesting: 0,
            indent,
        }
    }
}

impl Default for PrettyFormatter<'_> {
    fn default() -> Self {
        PrettyFormatter::new()
    }
}

impl Formatter for PrettyFormatter<'_> {
    #[inline]
    fn begin_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.array_nesting += 1;
        if self.array_nesting <= 1 {
            self.current_indent += 1;
        }
        self.has_value = false;
        writer.write_all(b"[")
    }

    #[inline]
    fn end_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if self.array_nesting <= 1 {
            self.current_indent -= 1;
            if self.has_value {
                writer.write_all(b"\n")?;
                indent(writer, self.current_indent, self.indent)?;
            }
        }

        self.array_nesting -= 1;
        writer.write_all(b"]")
    }

    #[inline]
    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if self.array_nesting > 1 {
            writer.write_all(if first { b"" } else { b", " })
        } else {
            writer.write_all(if first { b"\n" } else { b",\n" })?;
            indent(writer, self.current_indent, self.indent)
        }
    }

    #[inline]
    fn end_array_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.has_value = true;
        Ok(())
    }

    #[inline]
    fn begin_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent += 1;
        self.has_value = false;
        writer.write_all(b"{")
    }

    #[inline]
    fn end_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent -= 1;

        if self.has_value {
            writer.write_all(b"\n")?;
            indent(writer, self.current_indent, self.indent)?;
        }

        writer.write_all(b"}")
    }

    #[inline]
    fn begin_object_key<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(if first { b"\n" } else { b",\n" })?;
        indent(writer, self.current_indent, self.indent)
    }

    #[inline]
    fn begin_object_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b": ")
    }

    #[inline]
    fn end_object_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.has_value = true;
        Ok(())
    }
}
