const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const PAD: char = '=';
const GROUP: usize = 3;
const SIX_BITS: u32 = 0x3f;
const SHIFTS: [u32; 4] = [18, 12, 6, 0];

pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len().div_ceil(GROUP) * SHIFTS.len());
    for chunk in bytes.chunks(GROUP) {
        let group = chunk.iter().enumerate().fold(0u32, |group, (index, byte)| {
            group | u32::from(*byte) << (16 - 8 * index)
        });
        for (position, shift) in SHIFTS.iter().enumerate() {
            if position > chunk.len() {
                text.push(PAD);
            } else {
                text.push(char::from(ALPHABET[((group >> shift) & SIX_BITS) as usize]));
            }
        }
    }
    text
}
