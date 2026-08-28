//! Structural-parser regressions for lazy optional-tag validation.

use icc_profile::Profile;

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn profile_with_tags(tags: &[([u8; 4], &[u8])]) -> Vec<u8> {
    let mut bytes = vec![0; 132 + tags.len() * 12];
    bytes[8..12].copy_from_slice(&[4, 0, 0, 0]);
    bytes[12..16].copy_from_slice(b"mntr");
    bytes[16..20].copy_from_slice(b"GRAY");
    bytes[20..24].copy_from_slice(b"XYZ ");
    bytes[36..40].copy_from_slice(b"acsp");
    put_u32(&mut bytes, 64, 1);
    put_u32(&mut bytes, 128, tags.len() as u32);
    for (index, (signature, payload)) in tags.iter().enumerate() {
        let offset = (bytes.len() + 3) & !3;
        bytes.resize(offset + payload.len(), 0);
        bytes[offset..offset + payload.len()].copy_from_slice(payload);
        let entry = 132 + index * 12;
        bytes[entry..entry + 4].copy_from_slice(signature);
        put_u32(&mut bytes, entry + 4, offset as u32);
        put_u32(&mut bytes, entry + 8, payload.len() as u32);
    }
    let length = bytes.len() as u32;
    put_u32(&mut bytes, 0, length);
    bytes
}

fn identity_chad() -> [u8; 44] {
    let mut tag = [0; 44];
    tag[..4].copy_from_slice(b"sf32");
    for index in 0..9 {
        put_u32(
            &mut tag,
            8 + index * 4,
            if index % 4 == 0 { 65536 } else { 0 },
        );
    }
    tag
}

#[test]
fn parsed_valid_chad_is_lazy_but_visible() {
    let profile = profile_with_tags(&[(*b"chad", &identity_chad())]);
    let parsed = Profile::parse(&profile).unwrap();
    assert_eq!(
        parsed.chromatic_adaptation_checked().unwrap(),
        parsed.chromatic_adaptation().map(|_| identity_matrix())
    );
}

#[test]
fn malformed_lazy_chad_fails_when_checked() {
    let profile = profile_with_tags(&[(*b"chad", &[0; 4])]);
    let parsed = Profile::parse(&profile).unwrap();
    assert!(parsed.chromatic_adaptation_checked().is_err());
}

fn identity_matrix() -> [[f32; 3]; 3] {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}
