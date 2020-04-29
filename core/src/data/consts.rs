pub const TOKEN_STATEMENT: u8 = 0xff;
pub const TOKEN_FUNCTION: u8 = 0xfe;
pub const TOKEN_EXTERNAL: u8 = 0xfe;
pub const TOKEN_FILENAMES: u8 = 0xfd;

pub const TOKEN_VALUE_SCALAR: u8 = 0xe0;
pub const TOKEN_VALUE_STRUCTURAL: u8 = 0xe1;
pub const TOKEN_VALUE_ARRAY_LIKE: u8 = 0xe2;

pub const TOKEN_DATA_UNSUPPORTED: u8 = 0x10;
pub const TOKEN_DATA_I8: u8 = 0x11;
pub const TOKEN_DATA_I16: u8 = 0x12;
pub const TOKEN_DATA_I32: u8 = 0x13;
pub const TOKEN_DATA_I64: u8 = 0x14;
pub const TOKEN_DATA_U8: u8 = 0x15;
pub const TOKEN_DATA_U16: u8 = 0x16;
pub const TOKEN_DATA_U32: u8 = 0x17;
pub const TOKEN_DATA_U64: u8 = 0x18;
pub const TOKEN_DATA_F32: u8 = 0x19;
pub const TOKEN_DATA_F64: u8 = 0x20;
pub const TOKEN_DATA_BOOL: u8 = 0x21;

pub const META: u8 = 0x60;
pub const META_ARG: u8 = 0x61;
pub const META_RET: u8 = 0x62;
pub const META_CALL: u8 = 0x64;
