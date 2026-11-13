use icc_profile::iccprofile::{Data, ICCNumber, S15Fixed16Number};

fn raw_of(n: &S15Fixed16Number) -> i64 {
    i64::from(n.int()) * 65536 + i64::from(n.decimal())
}

#[test]
fn signed_raw_xyz_and_array_decode_exactly() {
    let values = [
        i32::MIN,
        i32::MIN + 1,
        -98304,
        -65536,
        -32768,
        -1,
        0,
        1,
        32767,
        32768,
        65535,
        65536,
        98304,
        i32::MAX - 1,
        i32::MAX,
    ];
    for raw in values {
        let expected = f64::from(raw) / 65536.0;
        let mut bytes = vec![0; 3]; // exercise a nonzero public-reader offset
        for _ in 0..3 {
            bytes.extend_from_slice(&raw.to_be_bytes());
        }
        let xyz = Data::xyz_number(&bytes, 3).unwrap();
        for n in [&xyz.x, &xyz.y, &xyz.z] {
            assert_eq!(n.as_f64(), expected);
            assert_eq!(n.as_f32(), expected as f32);
            assert_eq!(raw_of(n), i64::from(raw));
            assert_eq!(n.int(), raw.div_euclid(65536));
            assert_eq!(n.decimal(), raw.rem_euclid(65536) as u32);
        }
        let mut tag = b"sf32\0\0\0\0".to_vec();
        tag.extend_from_slice(&raw.to_be_bytes());
        let Data::S15Fixed16NumberArray(numbers) =
            Data::get("sf32", &tag, tag.len(), 0x04000000).unwrap()
        else {
            panic!()
        };
        assert_eq!(numbers.len(), 1);
        assert_eq!(numbers[0].as_f64(), expected);
        assert_eq!(raw_of(&S15Fixed16Number::new(expected)), i64::from(raw));
        assert_eq!(
            raw_of(&S15Fixed16Number::checked_new(expected).unwrap()),
            i64::from(raw)
        );
    }
}

#[test]
fn unsigned_raw_array_decode_exactly() {
    for raw in [0u32, 1, 32768, 65535, 65536, 98304, 0xffff0000, u32::MAX] {
        let mut tag = b"uf32\0\0\0\0".to_vec();
        tag.extend_from_slice(&raw.to_be_bytes());
        let Data::U16Fixed16NumberArray(numbers) =
            Data::get("uf32", &tag, tag.len(), 0x04000000).unwrap()
        else {
            panic!()
        };
        let n = &numbers[0];
        let expected = f64::from(raw) / 65536.0;
        assert_eq!(n.as_f64(), expected);
        assert_eq!(n.as_f32(), expected as f32);
        assert_eq!(n.int(), (raw / 65536) as i32);
        assert_eq!(n.decimal(), raw % 65536);
    }
}

#[test]
fn construction_floors_fraction_and_checks_exact_range() {
    for value in [-1.1, -0.1, -1e-20, -0.0, 0.0, 0.1, 0.5, 1.0, 1.1] {
        let raw = (value * 65536.0_f64).floor() as i64;
        assert_eq!(raw_of(&S15Fixed16Number::new(value)), raw);
        assert_eq!(raw_of(&S15Fixed16Number::checked_new(value).unwrap()), raw);
    }
    let max = f64::from(i32::MAX) / 65536.0;
    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -32768.000001,
        max + 0.000001,
        32768.0,
    ] {
        assert!(S15Fixed16Number::checked_new(value).is_none());
        // Verify the legacy constructor's invalid-input compatibility at field level.
        let old_integer = value.floor() as i16;
        let old_decimal = ((value - value.floor()) * 65535.0) as u16;
        let n = S15Fixed16Number::new(value);
        assert_eq!(n.int(), i32::from(old_integer));
        assert_eq!(n.decimal(), u32::from(old_decimal));
    }
}

#[test]
fn all_fraction_bits_round_trip_at_signed_extremes_and_zero() {
    for integer in [i16::MIN, -1, 0, 1, i16::MAX] {
        for fraction in 0..=u16::MAX {
            let raw = i64::from(integer) * 65536 + i64::from(fraction);
            let value = raw as f64 / 65536.0;
            assert_eq!(raw_of(&S15Fixed16Number::new(value)), raw);
        }
    }
}
