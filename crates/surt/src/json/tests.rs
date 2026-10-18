use super::base64;

#[test]
fn base64_pads_each_partial_group_as_rfc_4648_does() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64(b"foob"), "Zm9vYg==");
    assert_eq!(base64(b"fooba"), "Zm9vYmE=");
    assert_eq!(base64(b"foobar"), "Zm9vYmFy");
}

#[test]
fn base64_uses_the_standard_alphabet_for_every_byte_value() {
    assert_eq!(base64(&[0xff, 0xfe, 0x00, b'A']), "//4AQQ==");
    assert_eq!(base64(&[0xfb, 0xef, 0xbe]), "++++");
}
