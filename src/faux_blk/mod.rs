pub const FILE_READ: u16 = 1 << 1;
pub const FILE_WRITE: u16 = 1 << 2;

pub const FILE_OPEN_FLAG: u16  = 1 << 3;
pub const FILE_CLOSE_FLAG: u16 = 1 << 4;

pub const FILE_WRITE_CONTENTS_FLAG: u16 = 1 << 5;

pub const FILE_STATE_FLAG: u16 = 1 << 6;

pub const STATE_OPEN: u16   = 1 << 3;
pub const STATE_CLOSED: u16 = 1 << 4;

pub const STATE_SUCCESS: u16 = 1 << 5;
pub const STATE_FAIL: u16 = 1 << 6;
