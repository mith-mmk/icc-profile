//! Bounds regressions for legacy ASCII readers after the bin-rs range correction.
use icc_profile::iccprofile::{Data, DecodedICCProfile};

fn tag(kind: &[u8; 4], size: usize) -> Vec<u8> {
    let mut data = vec![0; size];
    data[..4].copy_from_slice(kind);
    data
}

fn profile(payload: &[u8], version: u32) -> Vec<u8> {
    let mut data = vec![0; 144];
    data[8..12].copy_from_slice(&version.to_be_bytes());
    data[12..16].copy_from_slice(b"mntr");
    data[16..20].copy_from_slice(b"GRAY");
    data[20..24].copy_from_slice(b"XYZ ");
    data[36..40].copy_from_slice(b"acsp");
    data[128..132].copy_from_slice(&1u32.to_be_bytes());
    data[132..136].copy_from_slice(b"tech");
    data[136..140].copy_from_slice(&144u32.to_be_bytes());
    data[140..144].copy_from_slice(&(payload.len() as u32).to_be_bytes());
    data.extend_from_slice(payload);
    let size = data.len() as u32;
    data[..4].copy_from_slice(&size.to_be_bytes());
    data
}

#[test]
fn truncated_signature_returns_error_directly_and_through_profile() {
    for version in [0x02000000, 0x04000000] {
        for size in 8..12 {
            let data = tag(b"sig ", size);
            assert!(Data::get("sig ", &data, size, version).is_err());
            assert!(DecodedICCProfile::new(&profile(&data, version)).is_err());
        }
    }
}

#[test]
fn truncated_description_returns_error_directly_and_through_profile() {
    for version in [0x02000000, 0x04000000] {
        for size in 8..12 {
            let data = tag(b"desc", size);
            assert!(Data::get("desc", &data, size, version).is_err());
            assert!(DecodedICCProfile::new(&profile(&data, version)).is_err());
        }
    }
}

#[test]
fn shifted_description_range_returns_error_directly_and_through_profile() {
    let mut data = tag(b"desc", 16);
    data[8..12].copy_from_slice(&4u32.to_be_bytes());
    data[12..16].copy_from_slice(b"ABC\0");
    // Keep the legacy branch threshold intact; this is an invalid major version.
    assert!(Data::get("desc", &data, 16, 0x40000000).is_err());
    assert!(DecodedICCProfile::new(&profile(&data, 0x40000000)).is_err());
}

#[test]
fn signature_range_and_description_boundary_keep_successful_outputs() {
    let mut data = tag(b"sig ", 12);
    data[8..12].copy_from_slice(b"RGB ");
    match Data::get("sig ", &data, 12, 0x04000000).unwrap() {
        Data::ASCII(value) => assert_eq!(value, "RGB "),
        other => panic!("unexpected value: {other:?}"),
    }
    for version in [0x02000000, 0x04000000] {
        let data = tag(b"desc", 12);
        match Data::get("desc", &data, 12, version).unwrap() {
            Data::ASCII(value) => assert_eq!(value, ""),
            other => panic!("unexpected value: {other:?}"),
        }
    }
}

#[test]
fn description_ascii_uses_its_own_field_and_preserves_nuls() {
    let mut data = tag(b"desc", 28);
    data[8..12].copy_from_slice(&4u32.to_be_bytes());
    data[12..16].copy_from_slice(b"old\0");
    data[16..20].copy_from_slice(b"en\0\0");
    match Data::get("desc", &data, data.len(), 0x40000000).unwrap() {
        Data::Descriptor(value) => {
            assert_eq!(value.ascii_string, "old\0");
            assert_eq!(value.lang, "en\0\0");
            assert_eq!(value.local_string, "");
        }
        other => panic!("unexpected value: {other:?}"),
    }
}

#[test]
fn short_declared_tag_length_returns_error_with_larger_backing_buffer() {
    for kind in ["text", "sig ", "desc", "data"] {
        let data = tag(kind.as_bytes().try_into().unwrap(), 16);
        for length in 0..8 {
            assert!(Data::get(kind, &data, length, 0x04000000).is_err());
        }
    }
    for kind in ["desc", "data"] {
        let data = tag(kind.as_bytes().try_into().unwrap(), 16);
        for length in 8..12 {
            assert!(Data::get(kind, &data, length, 0x04000000).is_err());
        }
    }
}

#[test]
fn data_payload_boundary_and_contents_keep_successful_outputs() {
    for payload in [b"".as_slice(), b"A\0B".as_slice()] {
        let mut data = tag(b"data", 12);
        data.extend_from_slice(payload);
        match Data::get("data", &data, data.len(), 0x04000000).unwrap() {
            Data::ASCII(value) => assert_eq!(value.as_bytes(), payload),
            other => panic!("unexpected value: {other:?}"),
        }
        data[8..12].copy_from_slice(&1u32.to_be_bytes());
        match Data::get("data", &data, data.len(), 0x04000000).unwrap() {
            Data::Binary(value) => assert_eq!(value, payload),
            other => panic!("unexpected value: {other:?}"),
        }
    }
}

#[test]
fn shifted_description_count_cannot_overflow_range_check() {
    let mut data = tag(b"desc", 16);
    data[8..12].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(Data::get("desc", &data, data.len(), 0x40000000).is_err());
}
