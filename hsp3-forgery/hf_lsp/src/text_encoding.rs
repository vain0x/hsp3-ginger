use encoding::all::WINDOWS_31J;
use encoding::codec::utf_8::UTF8Encoding;
use encoding::{DecoderTrap, Encoding, StringWriter};

pub(crate) fn decode_as_shift_jis_or_utf8(data: &[u8], out: &mut impl StringWriter) -> bool {
    // Windows-31J は CP932 の別名で、shift_jis とほぼ同じ。
    let shift_jis = WINDOWS_31J;

    shift_jis
        .decode_to(&data, DecoderTrap::Strict, out)
        .or_else(|_| UTF8Encoding.decode_to(&data, DecoderTrap::Strict, out))
        .is_ok()
}
