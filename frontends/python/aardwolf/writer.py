import struct

class Writer:
    def __init__(self, filename):
        self.fh_ = open(filename, 'wb')

    def close(self):
        self.fh_.close()

    def _write_packed(self, formatter, value):
        self.fh_.write(struct.pack(formatter, value))

    def write_str(self, value):
        self.fh_.write(value.encode('utf8'))

    def write_cstr(self, value):
        self.write_str(value + '\0')

    def write_token(self, value):
        self.write_u8(value)

    def write_u8(self, value):
        self._write_packed('B', value)

    def write_u16(self, value):
        self._write_packed('H', value)

    def write_u32(self, value):
        self._write_packed('I', value)

    def write_u64(self, value):
        self._write_packed('Q', value)
