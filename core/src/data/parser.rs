use std::collections::HashMap;
use std::io::{self, BufRead};

use super::access::Access;
use super::consts;
use super::module::Modules;
use super::statement::{Loc, Metadata, Statement};
use super::tests::{TestStatus, TestSuite};
use super::trace::{Trace, TraceItem};
use super::types::{FileId, FuncName, StmtId, TestName};
use super::values::Value;

#[derive(Debug)]
pub enum ParseError {
    UnexpectedByte {
        pos: usize,
        byte: u8,
        expected: Vec<u8>,
    },
    UnexpectedEof {
        n_bytes: usize,
    },
    ReadError {
        inner: io::Error,
    },
    InvalidFormat,
    UnsupportedVersion {
        version: u8,
    },
    InvalidUtf {
        value: Vec<u8>,
    },
    InvalidTestResult {
        value: String,
    },
}

impl ParseError {
    fn from_io(err: io::Error, buf: &[u8]) -> Self {
        match err.kind() {
            io::ErrorKind::UnexpectedEof => ParseError::UnexpectedEof { n_bytes: buf.len() },
            _ => ParseError::ReadError { inner: err },
        }
    }
}

pub struct Source<'a, R> {
    inner: &'a mut R,
    byte_pos: usize,
}

impl<'a, R> Source<'a, R> {
    pub fn new(source: &'a mut R) -> Self {
        Source {
            inner: source,
            byte_pos: 0,
        }
    }

    pub fn byte_pos(&self) -> usize {
        self.byte_pos
    }
}

impl<'a, R: BufRead> Source<'a, R> {
    pub fn read_exact(&mut self, buf: &mut [u8]) -> ParseResult<()> {
        self.inner
            .read_exact(buf)
            .map_err(|err| ParseError::from_io(err, buf))?;
        self.byte_pos += buf.len();
        Ok(())
    }

    pub fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> ParseResult<usize> {
        let n_bytes = self
            .inner
            .read_until(byte, buf)
            .map_err(|err| ParseError::from_io(err, buf))?;
        self.byte_pos += n_bytes;
        Ok(n_bytes)
    }

    pub fn read_line(&mut self, buf: &mut String) -> ParseResult<usize> {
        let n_bytes = self
            .inner
            .read_line(buf)
            .map_err(|err| ParseError::from_io(err, buf.as_bytes()))?;
        self.byte_pos += n_bytes;
        Ok(n_bytes)
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FormatKind {
    Static,
    Runtime,
}

pub struct Format {
    kind: FormatKind,
    version: u8,
}

fn version_from_ascii(byte: u8) -> u8 {
    byte.overflowing_sub('0' as u8).0
}

// Can be implemented as a normal function after const generics get stabilized.
// Tracking issue: https://github.com/rust-lang/rust/issues/44580.
macro_rules! read_n {
    ($source:expr, $n:expr) => {{
        let mut buf = [0u8; $n];
        $source.read_exact(&mut buf).map(|_| buf)
    }};
}

pub fn parse_module<'a, R: BufRead>(source: &'a mut R, modules: &mut Modules) -> ParseResult<()> {
    let source = &mut Source::new(source);
    let mut statements = HashMap::new();
    let mut function = String::new();

    match parse_header(source)? {
        Format {
            kind: FormatKind::Static,
            version: 1,
        } => {}
        Format {
            kind: FormatKind::Static,
            version,
        } => return Err(ParseError::UnsupportedVersion { version }),
        Format {
            kind: FormatKind::Runtime,
            ..
        } => return Err(ParseError::InvalidFormat),
    }

    while let Ok(token) = parse_u8(source) {
        match token {
            consts::TOKEN_STATEMENT => {
                let stmt = parse_stmt(source)?;
                statements.insert(stmt.id, stmt);
            }
            consts::TOKEN_FUNCTION => {
                let previous_function = std::mem::replace(&mut function, parse_cstr(source)?);
                if !statements.is_empty() {
                    modules.functions.insert(
                        FuncName::new(previous_function),
                        std::mem::replace(&mut statements, HashMap::new()),
                    );
                }
            }
            consts::TOKEN_FILENAMES => {
                let n_files = parse_u32(source)?;
                for _ in 0..n_files {
                    let file_id = parse_file_id(source)?;
                    let filepath = parse_cstr(source)?;
                    modules.files.insert(file_id, filepath);
                }
            }
            byte => {
                return Err(ParseError::UnexpectedByte {
                    pos: source.byte_pos(),
                    byte,
                    expected: vec![
                        consts::TOKEN_STATEMENT,
                        consts::TOKEN_FUNCTION,
                        consts::TOKEN_FILENAMES,
                    ],
                })
            }
        }
    }

    if !statements.is_empty() {
        modules
            .functions
            .insert(FuncName::new(function), statements);
    }

    Ok(())
}

pub fn parse_trace<'a, R: BufRead>(source: &'a mut R, trace: &mut Trace) -> ParseResult<()> {
    let source = &mut Source::new(source);

    match parse_header(source)? {
        Format {
            kind: FormatKind::Runtime,
            version: 1,
        } => {}
        Format {
            kind: FormatKind::Runtime,
            version,
        } => return Err(ParseError::UnsupportedVersion { version }),
        Format {
            kind: FormatKind::Static,
            ..
        } => return Err(ParseError::InvalidFormat),
    }

    while let Ok(token) = parse_u8(source) {
        let trace_item = match token {
            consts::TOKEN_STATEMENT => TraceItem::Statement(parse_stmt_id(source)?),
            consts::TOKEN_EXTERNAL => TraceItem::Test(TestName::new(parse_cstr(source)?)),
            consts::TOKEN_DATA_UNSUPPORTED => TraceItem::Value(Value::unsupported()),
            consts::TOKEN_DATA_I8 => TraceItem::Value(Value::signed(parse_i8(source)?)),
            consts::TOKEN_DATA_I16 => TraceItem::Value(Value::signed(parse_i16(source)?)),
            consts::TOKEN_DATA_I32 => TraceItem::Value(Value::signed(parse_i32(source)?)),
            consts::TOKEN_DATA_I64 => TraceItem::Value(Value::signed(parse_i64(source)?)),
            consts::TOKEN_DATA_U8 => TraceItem::Value(Value::unsigned(parse_u8(source)?)),
            consts::TOKEN_DATA_U16 => TraceItem::Value(Value::unsigned(parse_u16(source)?)),
            consts::TOKEN_DATA_U32 => TraceItem::Value(Value::unsigned(parse_u32(source)?)),
            consts::TOKEN_DATA_U64 => TraceItem::Value(Value::unsigned(parse_u64(source)?)),
            consts::TOKEN_DATA_F32 => TraceItem::Value(Value::floating(parse_f32(source)?)),
            consts::TOKEN_DATA_F64 => TraceItem::Value(Value::floating(parse_f64(source)?)),
            consts::TOKEN_DATA_BOOL => TraceItem::Value(Value::boolean(parse_boolean(source)?)),
            byte => {
                return Err(ParseError::UnexpectedByte {
                    pos: source.byte_pos(),
                    byte,
                    expected: vec![
                        consts::TOKEN_STATEMENT,
                        consts::TOKEN_EXTERNAL,
                        consts::TOKEN_DATA_UNSUPPORTED,
                        consts::TOKEN_DATA_I8,
                        consts::TOKEN_DATA_I16,
                        consts::TOKEN_DATA_I32,
                        consts::TOKEN_DATA_I64,
                        consts::TOKEN_DATA_U8,
                        consts::TOKEN_DATA_U16,
                        consts::TOKEN_DATA_U32,
                        consts::TOKEN_DATA_U64,
                        consts::TOKEN_DATA_F32,
                        consts::TOKEN_DATA_F64,
                        consts::TOKEN_DATA_BOOL,
                    ],
                })
            }
        };

        trace.trace.push(trace_item);
    }

    Ok(())
}

pub fn parse_test_suite<'a, R: BufRead>(
    source: &'a mut R,
    test_suite: &mut TestSuite,
) -> ParseResult<()> {
    let source = &mut Source::new(source);

    loop {
        let mut line = String::new();
        match source.read_line(&mut line)? {
            0 => return Ok(()),
            _ => match &line[0..6] {
                "PASS: " => test_suite
                    .tests
                    .insert(TestName::new(line[6..].trim()), TestStatus::Passed),
                "FAIL: " => test_suite
                    .tests
                    .insert(TestName::new(line[6..].trim()), TestStatus::Failed),
                result => {
                    return Err(ParseError::InvalidTestResult {
                        value: result.to_owned(),
                    })
                }
            },
        };
    }
}

fn parse_header<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<Format> {
    let buf = read_n!(source, 7).map_err(|_| ParseError::InvalidFormat)?;
    if &buf[0..6] == "AARD/S".as_bytes() {
        Ok(Format {
            kind: FormatKind::Static,
            version: version_from_ascii(buf[6]),
        })
    } else if &buf[0..6] == "AARD/D".as_bytes() {
        Ok(Format {
            kind: FormatKind::Runtime,
            version: version_from_ascii(buf[6]),
        })
    } else {
        Err(ParseError::InvalidFormat)
    }
}

fn parse_stmt<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<Statement> {
    let id = parse_stmt_id(source)?;

    let n_succ = parse_u8(source)?;
    let succ = parse_vec(source, n_succ, parse_stmt_id)?;

    let n_defs = parse_u8(source)?;
    let defs = parse_vec(source, n_defs, parse_access)?;

    let n_uses = parse_u8(source)?;
    let uses = parse_vec(source, n_uses, parse_access)?;

    let loc = parse_loc(source)?;
    let metadata = parse_metadata(source)?;

    Ok(Statement {
        id,
        succ,
        defs,
        uses,
        loc,
        metadata,
    })
}

fn parse_stmt_id<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<StmtId> {
    Ok(StmtId::new(parse_file_id(source)?, parse_u64(source)?))
}

fn parse_file_id<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<FileId> {
    Ok(FileId::new(parse_u64(source)?))
}

fn parse_access<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<Access> {
    match parse_u8(source)? {
        consts::TOKEN_VALUE_SCALAR => Ok(Access::Scalar(parse_u64(source)?)),
        consts::TOKEN_VALUE_STRUCTURAL => Ok(Access::Structural(
            Box::new(parse_access(source)?),
            Box::new(parse_access(source)?),
        )),
        consts::TOKEN_VALUE_ARRAY_LIKE => {
            let base = parse_access(source)?;
            let index_count = parse_u32(source)?;
            let index = parse_vec(source, index_count, parse_access)?;
            Ok(Access::ArrayLike(Box::new(base), index))
        }
        byte => Err(ParseError::UnexpectedByte {
            pos: source.byte_pos(),
            byte,
            expected: vec![
                consts::TOKEN_VALUE_SCALAR,
                consts::TOKEN_VALUE_STRUCTURAL,
                consts::TOKEN_VALUE_ARRAY_LIKE,
            ],
        }),
    }
}

fn parse_loc<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<Loc> {
    Ok(Loc {
        file_id: parse_file_id(source)?,
        line_begin: parse_u32(source)?,
        col_begin: parse_u32(source)?,
        line_end: parse_u32(source)?,
        col_end: parse_u32(source)?,
    })
}

fn parse_metadata<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<Metadata> {
    Ok(Metadata::new(parse_u8(source)?))
}

fn parse_vec<'a, R: BufRead, T, C, F>(
    source: &mut Source<'a, R>,
    count: C,
    parser: F,
) -> ParseResult<Vec<T>>
where
    C: Into<u64>,
    F: Fn(&mut Source<'a, R>) -> ParseResult<T>,
{
    let mut result = Vec::new();
    for _ in 0..count.into() {
        result.push(parser(source)?);
    }
    Ok(result)
}

fn parse_i8<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<i8> {
    Ok(i8::from_ne_bytes(read_n!(source, 1)?))
}

fn parse_i16<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<i16> {
    Ok(i16::from_ne_bytes(read_n!(source, 2)?))
}

fn parse_i32<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<i32> {
    Ok(i32::from_ne_bytes(read_n!(source, 4)?))
}

fn parse_i64<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<i64> {
    Ok(i64::from_ne_bytes(read_n!(source, 8)?))
}

fn parse_u8<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<u8> {
    Ok(u8::from_ne_bytes(read_n!(source, 1)?))
}

fn parse_u16<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<u16> {
    Ok(u16::from_ne_bytes(read_n!(source, 2)?))
}

fn parse_u32<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<u32> {
    Ok(u32::from_ne_bytes(read_n!(source, 4)?))
}

fn parse_u64<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<u64> {
    Ok(u64::from_ne_bytes(read_n!(source, 8)?))
}

fn parse_f32<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<f32> {
    Ok(f32::from_ne_bytes(read_n!(source, 4)?))
}

fn parse_f64<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<f64> {
    Ok(f64::from_ne_bytes(read_n!(source, 8)?))
}

fn parse_boolean<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<bool> {
    let buf = read_n!(source, 1)?;
    Ok(buf[0] > 0)
}

fn parse_cstr<'a, R: BufRead>(source: &mut Source<'a, R>) -> ParseResult<String> {
    let mut buf = Vec::new();
    source.read_until(0x0, &mut buf)?;

    // Remove null terminator
    buf.pop();

    String::from_utf8(buf).map_err(|err| ParseError::InvalidUtf {
        value: err.into_bytes(),
    })
}
