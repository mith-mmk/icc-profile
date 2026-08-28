//! E1 route metadata and executable class/model gates.

use icc_profile::{
    Profile, RenderingIntent, RouteModel, TransformDirection, TransformError, TransformLimits,
    TransformOptions,
};

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn xyz(x: f32, y: f32, z: f32) -> Vec<u8> {
    let mut tag = vec![0; 20];
    tag[..4].copy_from_slice(b"XYZ ");
    for (index, value) in [x, y, z].into_iter().enumerate() {
        put_u32(
            &mut tag,
            8 + index * 4,
            (value * 65536.0).round() as i32 as u32,
        );
    }
    tag
}

fn gamma() -> Vec<u8> {
    let mut tag = vec![0; 16];
    tag[..4].copy_from_slice(b"curv");
    put_u32(&mut tag, 8, 1);
    tag[12..14].copy_from_slice(&256u16.to_be_bytes());
    tag
}

fn matrix_profile(class: [u8; 4], version: [u8; 4], space: [u8; 4]) -> Vec<u8> {
    let tags = [
        (*b"rXYZ", xyz(0.9642, 0.0, 0.0)),
        (*b"gXYZ", xyz(0.0, 1.0, 0.0)),
        (*b"bXYZ", xyz(0.0, 0.0, 0.8249)),
        (*b"rTRC", gamma()),
        (*b"gTRC", gamma()),
        (*b"bTRC", gamma()),
    ];
    let mut bytes = vec![0; 132 + tags.len() * 12];
    bytes[8..12].copy_from_slice(&version);
    bytes[12..16].copy_from_slice(&class);
    bytes[16..20].copy_from_slice(&space);
    bytes[20..24].copy_from_slice(b"XYZ ");
    bytes[36..40].copy_from_slice(b"acsp");
    put_u32(&mut bytes, 64, 1);
    put_u32(&mut bytes, 128, tags.len() as u32);
    let mut offset = bytes.len();
    for (index, (signature, tag)) in tags.into_iter().enumerate() {
        offset = (offset + 3) & !3;
        bytes.resize(offset + tag.len(), 0);
        bytes[offset..offset + tag.len()].copy_from_slice(&tag);
        let entry = 132 + index * 12;
        bytes[entry..entry + 4].copy_from_slice(&signature);
        put_u32(&mut bytes, entry + 4, offset as u32);
        put_u32(&mut bytes, entry + 8, tag.len() as u32);
        offset += tag.len();
    }
    let length = bytes.len() as u32;
    put_u32(&mut bytes, 0, length);
    bytes
}

fn identity_mft2() -> Vec<u8> {
    let mut tag = vec![0; 124];
    tag[..4].copy_from_slice(b"mft2");
    tag[8] = 3;
    tag[9] = 3;
    tag[10] = 2;
    for index in 0..9 {
        put_u32(
            &mut tag,
            12 + index * 4,
            if index % 4 == 0 { 0x0001_0000 } else { 0 },
        );
    }
    tag[48..50].copy_from_slice(&2u16.to_be_bytes());
    tag[50..52].copy_from_slice(&2u16.to_be_bytes());
    let mut offset = 52;
    for _ in 0..3 {
        tag[offset..offset + 2].copy_from_slice(&0u16.to_be_bytes());
        tag[offset + 2..offset + 4].copy_from_slice(&u16::MAX.to_be_bytes());
        offset += 4;
    }
    for x in 0..2u16 {
        for y in 0..2u16 {
            for z in 0..2u16 {
                for value in [x, y, z] {
                    tag[offset..offset + 2]
                        .copy_from_slice(&(if value == 0 { 0 } else { u16::MAX }).to_be_bytes());
                    offset += 2;
                }
            }
        }
    }
    for _ in 0..3 {
        tag[offset..offset + 2].copy_from_slice(&0u16.to_be_bytes());
        tag[offset + 2..offset + 4].copy_from_slice(&u16::MAX.to_be_bytes());
        offset += 4;
    }
    tag
}

fn lut_profile(class: [u8; 4], version: [u8; 4], tag_name: [u8; 4]) -> Vec<u8> {
    let tag = identity_mft2();
    let mut bytes = vec![0; 144];
    bytes[8..12].copy_from_slice(&version);
    bytes[12..16].copy_from_slice(&class);
    bytes[16..20].copy_from_slice(b"RGB ");
    bytes[20..24].copy_from_slice(b"XYZ ");
    bytes[36..40].copy_from_slice(b"acsp");
    put_u32(&mut bytes, 64, 1);
    put_u32(&mut bytes, 128, 1);
    bytes.extend_from_slice(&tag);
    bytes[132..136].copy_from_slice(&tag_name);
    put_u32(&mut bytes, 136, 144);
    put_u32(&mut bytes, 140, tag.len() as u32);
    let length = bytes.len() as u32;
    put_u32(&mut bytes, 0, length);
    bytes
}

#[test]
fn raw_header_and_selected_fallback_are_retained_in_compiled_routes() {
    let bytes = lut_profile(*b"mntr", [4, 32, 0, 0], *b"A2B0");
    let profile = Profile::parse(&bytes).unwrap();
    assert_eq!(profile.raw_version(), [4, 32, 0, 0]);
    assert_eq!(profile.raw_device_class(), *b"mntr");

    let compiled = profile
        .compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::RelativeColorimetric,
            TransformLimits::default(),
        )
        .unwrap();
    let info = compiled.route_info();
    assert_eq!(
        info.requested_intent(),
        RenderingIntent::RelativeColorimetric
    );
    assert_eq!(info.selected_tag(), Some(*b"A2B0"));
    assert_eq!(info.model(), RouteModel::Lut);
    assert!(info.used_fallback());
    assert_eq!(compiled.raw_version(), [4, 32, 0, 0]);
    assert_eq!(compiled.raw_device_class(), *b"mntr");
}

#[test]
fn transform_retains_both_immutable_route_descriptions() {
    let bytes = matrix_profile(*b"mntr", [2, 0, 0, 0], *b"RGB ");
    let profile = Profile::parse(&bytes).unwrap();
    let transform = TransformOptions {
        rendering_intent: RenderingIntent::RelativeColorimetric,
        ..TransformOptions::default()
    };
    let transform = icc_profile::Transform::new(&profile, &profile, transform).unwrap();
    assert_eq!(transform.input_raw_version(), [2, 0, 0, 0]);
    assert_eq!(transform.output_raw_device_class(), *b"mntr");
    assert_eq!(transform.input_route_info().model(), RouteModel::Matrix);
    assert_eq!(transform.output_route_info().selected_tag(), None);
    assert!(transform.input_route_info().used_fallback());
    assert!(transform.output_route_info().used_fallback());
}

#[test]
fn unsupported_class_model_and_version_fail_before_compilation() {
    for (class, space) in [
        (*b"link", *b"RGB "),
        (*b"abst", *b"RGB "),
        (*b"nmcl", *b"RGB "),
    ] {
        let profile = Profile::parse(&matrix_profile(class, [4, 0, 0, 0], space)).unwrap();
        assert!(matches!(
            profile.compile(
                TransformDirection::DeviceToPcs,
                RenderingIntent::RelativeColorimetric,
                TransformLimits::default(),
            ),
            Err(TransformError::UnsupportedProfileFeature(_))
        ));
    }
    let profile = Profile::parse(&matrix_profile(*b"mntr", [3, 0, 0, 0], *b"RGB ")).unwrap();
    assert!(matches!(
        profile.compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::RelativeColorimetric,
            TransformLimits::default(),
        ),
        Err(TransformError::UnsupportedProfileFeature(_))
    ));
}

#[test]
fn class_model_matrix_rules_are_explicit_but_spac_lut_is_supported() {
    let prtr_rgb = Profile::parse(&matrix_profile(*b"prtr", [4, 0, 0, 0], *b"RGB ")).unwrap();
    assert!(matches!(
        prtr_rgb.compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::RelativeColorimetric,
            TransformLimits::default(),
        ),
        Err(TransformError::UnsupportedProfileFeature(_))
    ));
    let spac = Profile::parse(&lut_profile(*b"spac", [4, 0, 0, 0], *b"A2B0")).unwrap();
    assert!(spac
        .compile(
            TransformDirection::DeviceToPcs,
            RenderingIntent::RelativeColorimetric,
            TransformLimits::default(),
        )
        .is_ok());
}
