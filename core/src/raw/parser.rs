use std::collections::HashMap;
use std::fmt;
use std::io;
use std::io::prelude::*;

use crate::raw::data::*;

const TOKEN_STATEMENT: u8 = 0xff;
const TOKEN_FUNCTION: u8 = 0xfe;
const TOKEN_EXTERNAL: u8 = 0xfe;
const TOKEN_FILENAMES: u8 = 0xfd;

const TOKEN_VALUE_SCALAR: u8 = 0xe0;
const TOKEN_VALUE_STRUCTURAL: u8 = 0xe1;
const TOKEN_VALUE_ARRAY_LIKE: u8 = 0xe2;

const TOKEN_DATA_UNSUPPORTED: u8 = 0x10;
const TOKEN_DATA_I8: u8 = 0x11;
const TOKEN_DATA_I16: u8 = 0x12;
const TOKEN_DATA_I32: u8 = 0x13;
const TOKEN_DATA_I64: u8 = 0x14;
const TOKEN_DATA_U8: u8 = 0x15;
const TOKEN_DATA_U16: u8 = 0x16;
const TOKEN_DATA_U32: u8 = 0x17;
const TOKEN_DATA_U64: u8 = 0x18;
const TOKEN_DATA_F32: u8 = 0x19;
const TOKEN_DATA_F64: u8 = 0x20;

pub enum ParseError {
    UnexpectedByte,
    UnexpectedEof,
    ReadError,
    UnknownFormat,
}

impl From<io::Error> for ParseError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::UnexpectedEof => ParseError::UnexpectedEof,
            _ => ParseError::ReadError,
        }
    }
}

impl From<std::string::FromUtf8Error> for ParseError {
    fn from(_: std::string::FromUtf8Error) -> Self {
        ParseError::UnexpectedByte
    }
}

impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error: ")?;

        match self {
            ParseError::UnexpectedByte => write!(f, "unexpected byte"),
            ParseError::UnexpectedEof => write!(f, "unexpected eof"),
            ParseError::ReadError => write!(f, "read error"),
            ParseError::UnknownFormat => write!(f, "unknown format"),
        }
    }
}

enum Format {
    Static(u8),
    Dynamic(u8),
}

struct DataParser<'a, R: BufRead> {
    source: &'a mut R,
}

impl<'a, R: BufRead> DataParser<'a, R> {
    fn new(source: &'a mut R) -> Self {
        DataParser { source }
    }

    fn parse_static(&mut self) -> Result<StaticData, ParseError> {
        let mut statements: HashMap<u64, Statement> = HashMap::new();
        let mut functions: HashMap<String, HashMap<u64, Statement>> = HashMap::new();
        let mut files: HashMap<u32, String> = HashMap::new();

        let mut function = String::new();

        match self.parse_header()? {
            Format::Static(1) => {}
            _ => return Err(ParseError::UnknownFormat),
        }

        while let Ok(token) = self.parse_u8() {
            match token {
                TOKEN_STATEMENT => {
                    let stmt = self.parse_stmt()?;
                    statements.insert(stmt.id, stmt);
                }
                TOKEN_FUNCTION => {
                    let previous_function = std::mem::replace(&mut function, self.parse_cstr()?);
                    if !statements.is_empty() {
                        functions.insert(
                            previous_function,
                            std::mem::replace(&mut statements, HashMap::new()),
                        );
                    }
                }
                TOKEN_FILENAMES => {
                    let n_files = self.parse_u32()?;
                    for _ in 0..n_files {
                        let file_id = self.parse_u32()?;
                        let filepath = self.parse_cstr()?;
                        files.insert(file_id, filepath);
                    }
                }
                _ => return Err(ParseError::UnexpectedByte),
            }
        }

        if !statements.is_empty() {
            functions.insert(function, statements);
        }

        Ok(StaticData { functions, files })
    }

    fn parse_dynamic(&mut self) -> Result<DynamicData, ParseError> {
        let mut trace: Vec<TraceItem> = Vec::new();

        match self.parse_header()? {
            Format::Dynamic(1) => {}
            _ => return Err(ParseError::UnknownFormat),
        }

        while let Ok(token) = self.parse_u8() {
            match token {
                TOKEN_STATEMENT => trace.push(TraceItem::Statement(self.parse_u64()?)),
                TOKEN_EXTERNAL => trace.push(TraceItem::External(self.parse_cstr()?)),
                TOKEN_DATA_UNSUPPORTED => trace.push(TraceItem::Data(VariableData::Unsupported)),
                TOKEN_DATA_I8 => trace.push(TraceItem::Data(VariableData::I8(self.parse_i8()?))),
                TOKEN_DATA_I16 => trace.push(TraceItem::Data(VariableData::I16(self.parse_i16()?))),
                TOKEN_DATA_I32 => trace.push(TraceItem::Data(VariableData::I32(self.parse_i32()?))),
                TOKEN_DATA_I64 => trace.push(TraceItem::Data(VariableData::I64(self.parse_i64()?))),
                TOKEN_DATA_U8 => trace.push(TraceItem::Data(VariableData::U8(self.parse_u8()?))),
                TOKEN_DATA_U16 => trace.push(TraceItem::Data(VariableData::U16(self.parse_u16()?))),
                TOKEN_DATA_U32 => trace.push(TraceItem::Data(VariableData::U32(self.parse_u32()?))),
                TOKEN_DATA_U64 => trace.push(TraceItem::Data(VariableData::U64(self.parse_u64()?))),
                TOKEN_DATA_F32 => trace.push(TraceItem::Data(VariableData::F32(self.parse_f32()?))),
                TOKEN_DATA_F64 => trace.push(TraceItem::Data(VariableData::F64(self.parse_f64()?))),
                _ => return Err(ParseError::UnexpectedByte),
            }
        }

        Ok(DynamicData { trace })
    }

    fn parse_tests(&mut self) -> Result<TestData, ParseError> {
        let mut tests: HashMap<String, TestStatus> = HashMap::new();

        let mut ch_buf = [0; 1];
        loop {
            match self.source.read_exact(&mut ch_buf) {
                Ok(_) => {
                    if (ch_buf[0] as char) != '"' {
                        return Err(ParseError::UnexpectedByte);
                    }
                }
                Err(err) => {
                    if err.kind() == io::ErrorKind::UnexpectedEof {
                        return Ok(TestData { tests });
                    } else {
                        return Err(ParseError::from(err));
                    }
                }
            }

            let name = self.parse_quoted_str()?;

            let mut buf = [0; 2];
            self.source.read_exact(&mut buf).map_err(ParseError::from)?;
            if buf != [':' as u8, ' ' as u8] {
                return Err(ParseError::UnexpectedByte);
            }

            let status = self.parse_test_status()?;

            let mut buf = Vec::new();
            self.source
                .read_until('\n' as u8, &mut buf)
                .map_err(ParseError::from)?;

            tests.insert(name, status);
        }
    }

    fn parse_header(&mut self) -> Result<Format, ParseError> {
        let mut buf = [0; 7];
        self.source.read_exact(&mut buf).map_err(ParseError::from)?;
        if buf[0..6] == *"AARD/S".as_bytes() {
            Ok(Format::Static(buf[6].overflowing_sub('0' as u8).0))
        } else if buf[0..6] == *"AARD/D".as_bytes() {
            Ok(Format::Dynamic(buf[6].overflowing_sub('0' as u8).0))
        } else {
            Err(ParseError::UnknownFormat)
        }
    }

    fn parse_stmt(&mut self) -> Result<Statement, ParseError> {
        let id = self.parse_u64()?;

        let n_succ = self.parse_u8()?;
        let succ = self.parse_vec(n_succ, |this| this.parse_u64())?;

        let n_defs = self.parse_u8()?;
        let defs = self.parse_vec(n_defs, |this| this.parse_access())?;

        let n_uses = self.parse_u8()?;
        let uses = self.parse_vec(n_uses, |this| this.parse_access())?;

        let loc = Loc {
            file_id: self.parse_u32()?,
            line_begin: self.parse_u32()?,
            col_begin: self.parse_u32()?,
            line_end: self.parse_u32()?,
            col_end: self.parse_u32()?,
        };

        let metadata = self.parse_u8()?;

        Ok(Statement {
            id,
            succ,
            defs,
            uses,
            loc,
            metadata,
        })
    }

    fn parse_access(&mut self) -> Result<Access, ParseError> {
        match self.parse_u8()? {
            TOKEN_VALUE_SCALAR => Ok(Access::Scalar(self.parse_u64()?)),
            TOKEN_VALUE_STRUCTURAL => Ok(Access::Structural(
                Box::new(self.parse_access()?),
                Box::new(self.parse_access()?),
            )),
            TOKEN_VALUE_ARRAY_LIKE => {
                let base = self.parse_access()?;
                let count = self.parse_u32()?;
                let index = self.parse_vec(count, |this| this.parse_access())?;
                Ok(Access::ArrayLike(Box::new(base), index))
            }
            _ => Err(ParseError::UnexpectedByte),
        }
    }

    fn parse_i8(&mut self) -> Result<i8, ParseError> {
        let mut buf = [0; 1];
        self.source.read_exact(&mut buf).map_err(ParseError::from)?;
        Ok(i8::from_ne_bytes(buf))
    }

    fn parse_i16(&mut self) -> Result<i16, ParseError> {
        let mut buf = [0; 2];
        self.source.read_exact(&mut buf).map_err(ParseError::from)?;
        Ok(i16::from_ne_bytes(buf))
    }

    fn parse_i32(&mut self) -> Result<i32, ParseError> {
        let mut buf = [0; 4];
        self.source.read_exact(&mut buf).map_err(ParseError::from)?;
        Ok(i32::from_ne_bytes(buf))
    }

    fn parse_i64(&mut self) -> Result<i64, ParseError> {
        let mut buf = [0; 8];
        self.source.read_exact(&mut buf).map_err(ParseError::from)?;
        Ok(i64::from_ne_bytes(buf))
    }

    fn parse_u8(&mut self) -> Result<u8, ParseError> {
        let mut buf = [0; 1];
        self.source.read_exact(&mut buf).map_err(ParseError::from)?;
        Ok(u8::from_ne_bytes(buf))
    }

    fn parse_u16(&mut self) -> Result<u16, ParseError> {
        let mut buf = [0; 2];
        self.source.read_exact(&mut buf).map_err(ParseError::from)?;
        Ok(u16::from_ne_bytes(buf))
    }

    fn parse_u32(&mut self) -> Result<u32, ParseError> {
        let mut buf = [0; 4];
        self.source.read_exact(&mut buf).map_err(ParseError::from)?;
        Ok(u32::from_ne_bytes(buf))
    }

    fn parse_u64(&mut self) -> Result<u64, ParseError> {
        let mut buf = [0; 8];
        self.source.read_exact(&mut buf).map_err(ParseError::from)?;
        Ok(u64::from_ne_bytes(buf))
    }

    fn parse_f32(&mut self) -> Result<f32, ParseError> {
        let mut buf = [0; 4];
        self.source.read_exact(&mut buf).map_err(ParseError::from)?;
        Ok(f32::from_ne_bytes(buf))
    }

    fn parse_f64(&mut self) -> Result<f64, ParseError> {
        let mut buf = [0; 8];
        self.source.read_exact(&mut buf).map_err(ParseError::from)?;
        Ok(f64::from_ne_bytes(buf))
    }

    fn parse_cstr(&mut self) -> Result<String, ParseError> {
        let mut buf = Vec::new();
        self.source
            .read_until(0x0, &mut buf)
            .map_err(ParseError::from)?;

        // Remove null terminator
        buf.pop();

        String::from_utf8(buf).map_err(ParseError::from)
    }

    fn parse_quoted_str(&mut self) -> Result<String, ParseError> {
        let mut buf = [0; 1];

        let mut output = String::new();
        let mut escape = false;

        loop {
            self.source.read_exact(&mut buf).map_err(ParseError::from)?;
            let ch = buf[0] as char;

            if !escape {
                if ch == '\\' {
                    escape = true;
                } else if ch == '"' {
                    return Ok(output);
                } else if ch == '\n' {
                    return Err(ParseError::UnexpectedByte);
                } else {
                    output.push(ch);
                }
            } else {
                output.push(ch);
            }
        }
    }

    fn parse_test_status(&mut self) -> Result<TestStatus, ParseError> {
        let mut buf = [0; 6];
        self.source.read_exact(&mut buf).map_err(ParseError::from)?;

        let data = String::from_utf8(Vec::from(&buf[..])).map_err(ParseError::from)?;

        match data.as_str() {
            "PASSED" => Ok(TestStatus::Passed),
            "FAILED" => Ok(TestStatus::Failed),
            _ => Err(ParseError::UnexpectedByte),
        }
    }

    fn parse_vec<T, C, F>(&mut self, count: C, parser: F) -> Result<Vec<T>, ParseError>
    where
        C: Into<u64>,
        F: Fn(&mut Self) -> Result<T, ParseError>,
    {
        let mut result = Vec::new();
        for _ in 0..count.into() {
            result.push(parser(self)?);
        }
        Ok(result)
    }
}

impl StaticData {
    pub fn parse<R: BufRead>(source: &mut R) -> Result<StaticData, ParseError> {
        DataParser::new(source).parse_static()
    }
}

impl DynamicData {
    pub fn parse<R: BufRead>(source: &mut R) -> Result<DynamicData, ParseError> {
        DataParser::new(source).parse_dynamic()
    }
}

impl TestData {
    pub fn parse<R: BufRead>(source: &mut R) -> Result<TestData, ParseError> {
        DataParser::new(source).parse_tests()
    }
}

impl Data {
    pub fn parse<R1: BufRead, R2: BufRead, R3: BufRead>(
        static_data_source: &mut R1,
        dynamic_data_source: &mut R2,
        test_data_source: &mut R3,
    ) -> Result<Data, ParseError> {
        let static_data = StaticData::parse(static_data_source);
        let dynamic_data = DynamicData::parse(dynamic_data_source);
        let test_data = TestData::parse(test_data_source);

        match (static_data, dynamic_data, test_data) {
            (Ok(static_data), Ok(dynamic_data), Ok(test_data)) => Ok(Data {
                static_data,
                dynamic_data,
                test_data,
            }),
            (Err(err), _, _) => Err(err),
            (_, Err(err), _) => Err(err),
            (_, _, Err(err)) => Err(err),
        }
    }
}
