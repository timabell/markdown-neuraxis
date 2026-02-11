pub struct WikiLink;

impl WikiLink {
    pub const OPEN: &'static [u8; 2] = b"[[";
    pub const CLOSE: &'static [u8; 2] = b"]]";
    pub const ALIAS: u8 = b'|';
}
