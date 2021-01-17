const LOOKUP_TABLE: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
];

pub fn hex_upper_nibble(byte: &u8) -> char {
    let idx = (*byte >> 4 & 0x0F) as usize;
    LOOKUP_TABLE[idx]
}

pub fn hex_lower_nibble(byte: &u8) -> char {
    let idx = (*byte & 0x0F) as usize;
    LOOKUP_TABLE[idx]
}
