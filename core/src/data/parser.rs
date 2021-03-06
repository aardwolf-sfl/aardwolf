use std::collections::HashMap;
use std::fmt;
use std::io::{self, BufRead};

use super::access::Access;
use super::consts;
use super::module::Modules;
use super::statement::{Loc, Metadata, Statement};
use super::tests::{TestStatus, TestSuite};
use super::trace::{Trace, TraceItem};
use super::types::{FileId, FuncName, StmtId};
use super::values::{IntoValue, Value, ValueRef, ValueType};
use super::Arenas;
use crate::arena::{P, S};

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
    InvalidData {
        reason: String,
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

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedByte {
                pos,
                byte,
                expected,
            } => {
                write!(
                    f,
                    "unexpected byte 0x{:x} at position {}, expected one of 0x{:x}",
                    byte, pos, expected[0]
                )?;
                for byte in expected.into_iter().skip(1) {
                    write!(f, ", 0x{:x}", byte)?;
                }
                Ok(())
            }
            ParseError::UnexpectedEof { n_bytes } => {
                write!(f, "unexpected end of file when readong {} bytes", n_bytes)
            }
            ParseError::ReadError { inner } => write!(f, "{}", inner),
            ParseError::InvalidFormat => write!(f, "invalid format of data file"),
            ParseError::UnsupportedVersion { version } => {
                write!(f, "unsupported version {}", version)
            }
            ParseError::InvalidUtf { value } => write!(
                f,
                "invalid utf-8 encoding ({})",
                value
                    .into_iter()
                    .map(|byte| format!("0x{:x}", byte))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            ParseError::InvalidTestResult { value } => {
                write!(f, "invalid test result \"{}\"", value)
            }
            ParseError::InvalidData { reason } => write!(f, "invalid data, reason: {}", reason),
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

struct Parser<'a, 'b, R> {
    source: Source<'a, R>,
    arenas: &'b mut Arenas,
    buffer: Vec<u8>,
}

impl<'a, 'b, R> Parser<'a, 'b, R> {
    pub fn new(source: &'a mut R, arenas: &'b mut Arenas) -> Self {
        Parser {
            source: Source::new(source),
            arenas,
            buffer: Vec::with_capacity(64),
        }
    }
}

macro_rules! extend_buffer {
    ($parser:expr, $num:expr) => {{
        let value = $num;
        $parser.buffer.extend_from_slice(&value.to_ne_bytes());
        value
    }};
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FormatKind {
    Static,
    Runtime,
}

pub struct Format {
    kind: FormatKind,
    version: u8,
}

pub(crate) fn parse_module<'a, 'b, R: BufRead>(
    source: &'a mut R,
    modules: &mut Modules,
    arenas: &'b mut Arenas,
) -> ParseResult<()> {
    let mut parser = Parser::new(source, arenas);
    let mut statements = HashMap::new();
    let mut func_ptr = None;

    match parser.parse_header()? {
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

    while let Ok(token) = parser.parse_u8() {
        match token {
            consts::TOKEN_STATEMENT => {
                if let Some(func) = func_ptr {
                    let (id, stmt) = parser.parse_stmt(func)?;
                    statements.insert(id, stmt);
                } else {
                    return Err(ParseError::InvalidData {
                        reason: "every statement must be inside a function".to_owned(),
                    });
                }
            }
            consts::TOKEN_FUNCTION => {
                // Register previously encountered function since we collected
                // all its statements.
                if !statements.is_empty() {
                    if let Some(func) = func_ptr {
                        modules
                            .functions
                            .insert(func, std::mem::replace(&mut statements, HashMap::new()));
                    } else {
                        return Err(ParseError::InvalidData {
                            reason: "there are statements that do not belong to any function"
                                .to_owned(),
                        });
                    }
                }

                let function = parser.parse_cstr()?;
                func_ptr = Some(parser.arenas.func.alloc(function));
            }
            consts::TOKEN_FILENAMES => {
                let n_files = parser.parse_u32()?;
                for _ in 0..n_files {
                    let file_id = parser.parse_file_id()?;
                    let filepath = parser.parse_cstr()?;
                    modules
                        .files
                        .insert(file_id, parser.arenas.file.alloc(filepath));
                }
            }
            byte => {
                return Err(ParseError::UnexpectedByte {
                    pos: parser.source.byte_pos(),
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
        if let Some(func) = func_ptr {
            modules.functions.insert(func, statements);
        } else {
            return Err(ParseError::InvalidData {
                reason: "there are statements that do not belong to any function".to_owned(),
            });
        }
    }

    Ok(())
}

pub(crate) fn parse_trace<'a, 'b, R: BufRead>(
    source: &'a mut R,
    trace: &mut Trace,
    arenas: &'b mut Arenas,
    ignore_corrupted: bool,
) -> ParseResult<()> {
    let mut parser = Parser::new(source, arenas);

    match parser.parse_header()? {
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

    macro_rules! try_parse {
        ($action:expr) => {
            match $action {
                Ok(value) => value,
                Err(error) => {
                    if ignore_corrupted {
                        continue;
                    } else {
                        return Err(error.into());
                    }
                }
            }
        };
    }

    while let Ok(token) = parser.parse_u8() {
        let trace_item = match token {
            consts::TOKEN_STATEMENT => TraceItem::Statement(try_parse!(parser.parse_stmt_id())),
            consts::TOKEN_EXTERNAL => {
                let parsed = try_parse!(parser.parse_cstr());
                TraceItem::Test(parser.arenas.test.alloc(parsed))
            }
            consts::TOKEN_DATA_UNSUPPORTED
            | consts::TOKEN_DATA_I8
            | consts::TOKEN_DATA_I16
            | consts::TOKEN_DATA_I32
            | consts::TOKEN_DATA_I64
            | consts::TOKEN_DATA_U8
            | consts::TOKEN_DATA_U16
            | consts::TOKEN_DATA_U32
            | consts::TOKEN_DATA_U64
            | consts::TOKEN_DATA_F32
            | consts::TOKEN_DATA_F64
            | consts::TOKEN_DATA_BOOL => TraceItem::Value(try_parse!(parser.parse_value(token))),
            byte => {
                if ignore_corrupted {
                    // Read next byte.
                    continue;
                } else {
                    return Err(ParseError::UnexpectedByte {
                        pos: parser.source.byte_pos(),
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
                    });
                }
            }
        };

        trace.trace.push(trace_item);
    }

    Ok(())
}

pub(crate) fn parse_test_suite<'a, 'b, R: BufRead>(
    source: &'a mut R,
    test_suite: &mut TestSuite,
    arenas: &'b mut Arenas,
) -> ParseResult<()> {
    let mut parser = Parser::new(source, arenas);

    loop {
        let mut line = String::new();
        match parser.source.read_line(&mut line)? {
            0 => return Ok(()),
            _ => match &line[0..6] {
                "PASS: " => test_suite.tests.insert(
                    parser.arenas.test.alloc(line[6..].trim()),
                    TestStatus::Passed,
                ),
                "FAIL: " => test_suite.tests.insert(
                    parser.arenas.test.alloc(line[6..].trim()),
                    TestStatus::Failed,
                ),
                result => {
                    return Err(ParseError::InvalidTestResult {
                        value: result.to_owned(),
                    })
                }
            },
        };
    }
}

fn version_from_ascii(byte: u8) -> u8 {
    byte.overflowing_sub('0' as u8).0
}

// Can be implemented as a normal function after const generics get stabilized.
// Tracking issue: https://github.com/rust-lang/rust/issues/44580.
macro_rules! read_n {
    ($parser:expr, $n:expr) => {{
        let mut buf = [0u8; $n];
        $parser.source.read_exact(&mut buf).map(|_| buf)
    }};
}

impl<'a, 'b, R: BufRead> Parser<'a, 'b, R> {
    fn parse_header(&mut self) -> ParseResult<Format> {
        let buf = read_n!(self, 7).map_err(|_| ParseError::InvalidFormat)?;
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

    fn parse_stmt(&mut self, func: S<FuncName>) -> ParseResult<(StmtId, P<Statement>)> {
        self.buffer.clear();
        let id = self.parse_stmt_id()?;
        let buffer = self.buffer.clone();

        let n_succ = self.parse_u8()?;
        let succ = self.parse_vec(n_succ, Self::parse_stmt_id)?;

        let n_defs = self.parse_u8()?;
        let defs = self.parse_vec(n_defs, Self::parse_access)?;
        let defs = defs.into_iter().collect();

        let n_uses = self.parse_u8()?;
        let uses = self.parse_vec(n_uses, Self::parse_access)?;
        let uses = uses.into_iter().collect();

        let loc = self.parse_loc()?;
        let metadata = self.parse_metadata()?;

        let ptr = self.arenas.stmt.alloc(
            Statement {
                id,
                succ,
                defs,
                uses,
                loc,
                metadata,
                func,
            },
            &buffer,
        );

        Ok((id, ptr))
    }

    fn parse_stmt_id(&mut self) -> ParseResult<StmtId> {
        let file_id = self.parse_file_id()?;
        let stmt_id = extend_buffer!(self, self.parse_u64()?);
        Ok(StmtId::new(self.arenas.stmt_id.get((file_id, stmt_id))))
    }

    fn parse_file_id(&mut self) -> ParseResult<FileId> {
        Ok(FileId::new(extend_buffer!(self, self.parse_u64()?)))
    }

    fn parse_access(&mut self) -> ParseResult<P<Access>> {
        self.buffer.clear();
        let access = self.parse_access_impl()?;
        Ok(self.arenas.access.alloc(access, &self.buffer))
    }

    fn parse_access_impl(&mut self) -> ParseResult<Access> {
        match extend_buffer!(self, self.parse_u8()?) {
            consts::TOKEN_VALUE_SCALAR => {
                Ok(Access::Scalar(extend_buffer!(self, self.parse_u64()?)))
            }
            consts::TOKEN_VALUE_STRUCTURAL => Ok(Access::Structural(
                Box::new(self.parse_access_impl()?),
                Box::new(self.parse_access_impl()?),
            )),
            consts::TOKEN_VALUE_ARRAY_LIKE => {
                let base = self.parse_access_impl()?;
                let index_count = extend_buffer!(self, self.parse_u32()?);
                let index = self.parse_vec(index_count, Self::parse_access_impl)?;
                Ok(Access::ArrayLike(Box::new(base), index))
            }
            byte => Err(ParseError::UnexpectedByte {
                pos: self.source.byte_pos(),
                byte,
                expected: vec![
                    consts::TOKEN_VALUE_SCALAR,
                    consts::TOKEN_VALUE_STRUCTURAL,
                    consts::TOKEN_VALUE_ARRAY_LIKE,
                ],
            }),
        }
    }

    fn parse_loc(&mut self) -> ParseResult<Loc> {
        Ok(Loc {
            file_id: self.parse_file_id()?,
            line_begin: self.parse_u32()?,
            col_begin: self.parse_u32()?,
            line_end: self.parse_u32()?,
            col_end: self.parse_u32()?,
        })
    }

    fn parse_metadata(&mut self) -> ParseResult<Metadata> {
        Ok(Metadata::new(self.parse_u8()?))
    }

    fn parse_value(&mut self, token: u8) -> ParseResult<ValueRef> {
        let value = match token {
            consts::TOKEN_DATA_UNSUPPORTED => self
                .arenas
                .value
                .alloc(Value::Unsupported, ValueType::Unsupported),
            consts::TOKEN_DATA_I8 => {
                let parsed = self.parse_i8()?;
                let (value, value_type) = parsed.into_value();
                self.arenas.value.alloc(value, value_type)
            }
            consts::TOKEN_DATA_I16 => {
                let parsed = self.parse_i16()?;
                let (value, value_type) = parsed.into_value();
                self.arenas.value.alloc(value, value_type)
            }
            consts::TOKEN_DATA_I32 => {
                let parsed = self.parse_i32()?;
                let (value, value_type) = parsed.into_value();
                self.arenas.value.alloc(value, value_type)
            }
            consts::TOKEN_DATA_I64 => {
                let parsed = self.parse_i64()?;
                let (value, value_type) = parsed.into_value();
                self.arenas.value.alloc(value, value_type)
            }
            consts::TOKEN_DATA_U8 => {
                let parsed = self.parse_u8()?;
                let (value, value_type) = parsed.into_value();
                self.arenas.value.alloc(value, value_type)
            }
            consts::TOKEN_DATA_U16 => {
                let parsed = self.parse_u16()?;
                let (value, value_type) = parsed.into_value();
                self.arenas.value.alloc(value, value_type)
            }
            consts::TOKEN_DATA_U32 => {
                let parsed = self.parse_u32()?;
                let (value, value_type) = parsed.into_value();
                self.arenas.value.alloc(value, value_type)
            }
            consts::TOKEN_DATA_U64 => {
                let parsed = self.parse_u64()?;
                let (value, value_type) = parsed.into_value();
                self.arenas.value.alloc(value, value_type)
            }
            consts::TOKEN_DATA_F32 => {
                let parsed = self.parse_f32()?;
                let (value, value_type) = parsed.into_value();
                self.arenas.value.alloc(value, value_type)
            }
            consts::TOKEN_DATA_F64 => {
                let parsed = self.parse_f64()?;
                let (value, value_type) = parsed.into_value();
                self.arenas.value.alloc(value, value_type)
            }
            consts::TOKEN_DATA_BOOL => {
                let parsed = self.parse_boolean()?;
                let (value, value_type) = parsed.into_value();
                self.arenas.value.alloc(value, value_type)
            }
            _ => unreachable!(),
        };

        Ok(value)
    }

    fn parse_vec<T, C, F>(&mut self, count: C, parser: F) -> ParseResult<Vec<T>>
    where
        C: Into<u64>,
        F: Fn(&mut Self) -> ParseResult<T>,
    {
        let mut result = Vec::new();
        for _ in 0..count.into() {
            result.push(parser(self)?);
        }
        Ok(result)
    }

    fn parse_i8(&mut self) -> ParseResult<i8> {
        Ok(i8::from_ne_bytes(read_n!(self, 1)?))
    }

    fn parse_i16(&mut self) -> ParseResult<i16> {
        Ok(i16::from_ne_bytes(read_n!(self, 2)?))
    }

    fn parse_i32(&mut self) -> ParseResult<i32> {
        Ok(i32::from_ne_bytes(read_n!(self, 4)?))
    }

    fn parse_i64(&mut self) -> ParseResult<i64> {
        Ok(i64::from_ne_bytes(read_n!(self, 8)?))
    }

    fn parse_u8(&mut self) -> ParseResult<u8> {
        Ok(u8::from_ne_bytes(read_n!(self, 1)?))
    }

    fn parse_u16(&mut self) -> ParseResult<u16> {
        Ok(u16::from_ne_bytes(read_n!(self, 2)?))
    }

    fn parse_u32(&mut self) -> ParseResult<u32> {
        Ok(u32::from_ne_bytes(read_n!(self, 4)?))
    }

    fn parse_u64(&mut self) -> ParseResult<u64> {
        Ok(u64::from_ne_bytes(read_n!(self, 8)?))
    }

    fn parse_f32(&mut self) -> ParseResult<f32> {
        Ok(f32::from_ne_bytes(read_n!(self, 4)?))
    }

    fn parse_f64(&mut self) -> ParseResult<f64> {
        Ok(f64::from_ne_bytes(read_n!(self, 8)?))
    }

    fn parse_boolean(&mut self) -> ParseResult<bool> {
        let buf = read_n!(self, 1)?;
        Ok(buf[0] > 0)
    }

    fn parse_cstr(&mut self) -> ParseResult<String> {
        let mut buf = Vec::new();
        self.source.read_until(0x0, &mut buf)?;

        // Remove null terminator
        buf.pop();

        String::from_utf8(buf).map_err(|err| ParseError::InvalidUtf {
            value: err.into_bytes(),
        })
    }
}
