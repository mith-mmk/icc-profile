use super::{ICCNumber, U1Fixed15Number, U8Fixed8Number};

#[test]
fn all_unsigned_16_bit_encodings_match_integer_arithmetic() {
    // These legacy types have no public constructors or parser paths.
    for raw in 0..=u16::MAX {
        let u1 = U1Fixed15Number { decimal: raw };
        assert_eq!(u1.as_f64(), f64::from(raw) / 32768.0);
        assert_eq!(u1.as_f32(), f32::from(raw) / 32768.0);
        assert_eq!(u1.int(), i32::from(raw / 32768));
        assert_eq!(u1.decimal(), u32::from(raw)); // historical raw-word contract
        let u8 = U8Fixed8Number {
            integer: (raw >> 8) as u8,
            decimal: raw as u8,
        };
        assert_eq!(u8.as_f64(), f64::from(raw) / 256.0);
        assert_eq!(u8.as_f32(), f32::from(raw) / 256.0);
        assert_eq!(u8.int(), i32::from(raw / 256));
        assert_eq!(u8.decimal(), u32::from(raw % 256));
    }
}
