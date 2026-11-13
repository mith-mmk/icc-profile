//! Bounded I/O and diagnostic output are separate from legacy payload decoding.
use icc_profile::iccprofile::{
    Data, Descriptor, LocalizedUnicode, Mft1, Mft2, MultiLocalizedUnicodeType,
    OneDimensionalCurvesType, ParametricCurve,
};
use icc_profile::utils::{
    decoded_print, decoded_print_with_limit, load, load_with_limit, read_with_limit,
};
use icc_profile::DecodedICCProfile;
use std::io::{self, ErrorKind, Read};

fn profile() -> DecodedICCProfile {
    DecodedICCProfile::new(&vec![0; 132]).unwrap()
}

#[test]
fn reads_exact_limit_and_rejects_one_more_without_truncated_success() {
    for limit in [0, 1, 3, 8191, 8192, 8193, 20_000] {
        let bytes = vec![41; limit];
        let output = read_with_limit(bytes.as_slice(), limit).unwrap();
        assert_eq!(output, bytes);
        assert!(output.capacity() <= limit);
        assert!(read_with_limit(vec![41; limit + 1].as_slice(), limit).is_err());
    }
    assert_eq!(read_with_limit(&b"abc"[..], usize::MAX).unwrap(), b"abc");
}

#[test]
fn growing_stream_reads_only_budget_and_one_probe_byte() {
    struct Growing {
        read: usize,
        calls: usize,
    }
    impl Read for Growing {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            self.calls += 1;
            let count = output.len().min(7);
            output[..count].fill(1);
            self.read += count;
            Ok(count) // No EOF, independently of any initial metadata.
        }
    }
    for limit in [0, 1, 17, 16385] {
        let mut reader = Growing { read: 0, calls: 0 };
        assert_eq!(
            read_with_limit(&mut reader, limit).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
        assert_eq!(reader.read, limit + 1);
        assert!(reader.calls <= limit / 7 + 2);
    }
}

#[test]
fn interrupted_reads_retry_and_other_errors_propagate() {
    struct Interrupted {
        first: bool,
    }
    impl Read for Interrupted {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            if self.first {
                self.first = false;
                Err(ErrorKind::Interrupted.into())
            } else {
                Err(ErrorKind::PermissionDenied.into())
            }
        }
    }
    assert_eq!(
        read_with_limit(Interrupted { first: true }, 10)
            .unwrap_err()
            .kind(),
        ErrorKind::PermissionDenied
    );
    struct BadCount;
    impl Read for BadCount {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Ok(usize::MAX)
        }
    }
    assert!(read_with_limit(BadCount, 10).is_err());
}

#[test]
fn file_loader_enforces_input_budget_and_keeps_legacy_signature() {
    let path = std::env::temp_dir().join(format!(
        "icc-bounded-{}-{}.icc",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _cleanup = Cleanup(path.clone());
    let data = vec![0; 132];
    std::fs::write(&path, &data).unwrap();
    let name = path.to_string_lossy().into_owned();
    assert_eq!(load_with_limit(name.clone(), 132).unwrap().data, data);
    assert_eq!(
        load_with_limit(name.clone(), 131).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(
        load_with_limit(name.clone(), 0).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(load(name).unwrap().data, data);
}

fn exact_output_limits(value: &DecodedICCProfile) -> String {
    let expected = decoded_print_with_limit(value, 100_000).unwrap();
    assert_eq!(
        decoded_print_with_limit(value, expected.len()).unwrap(),
        expected
    );
    assert_eq!(
        decoded_print_with_limit(value, expected.len() + 1).unwrap(),
        expected
    );
    for limit in [0, 1, expected.len() - 1] {
        assert_eq!(
            decoded_print_with_limit(value, limit).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }
    expected
}

#[test]
fn exact_output_budget_counts_utf8_and_debug_escapes() {
    let mut value = profile();
    value.create_date = "日付🦀".into();
    value
        .tags
        .insert("名🦀".into(), Data::ASCII("日本語🦀\n\r\t\0\"\\".into()));
    let expected = exact_output_limits(&value);
    assert!(expected.contains("日本語🦀"));
    assert!(expected.contains(r"\n\r\t\0\"));
    assert!(expected.len() > expected.chars().count());
    assert!(decoded_print_with_limit(&value, expected.chars().count()).is_err());
}

#[test]
fn output_is_deterministic_and_chad_label_does_not_assume_d65() {
    let mut first = profile();
    let mut second = profile();
    for name in ["zzzz", "desc", "chad", "aaaa"] {
        first.tags.insert(name.into(), Data::ASCII(name.into()));
    }
    for name in ["aaaa", "chad", "desc", "zzzz"] {
        second.tags.insert(name.into(), Data::ASCII(name.into()));
    }
    for (left, right) in [
        (
            decoded_print(&first, 0).unwrap(),
            decoded_print(&second, 0).unwrap(),
        ),
        (exact_output_limits(&first), exact_output_limits(&second)),
    ] {
        assert_eq!(left, right);
        assert!(left.contains("Chromatic adaptation matrix"));
        assert!(!left.contains("D65 to D50"));
        let positions: Vec<_> = ["aaaa", "chad", "desc", "zzzz"]
            .iter()
            .map(|name| left.find(&format!("Tag name {name}\n")).unwrap())
            .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }
}

#[test]
fn malformed_public_payload_counts_do_not_drive_indexing() {
    let values = [
        Data::Lut8(Mft1 {
            input_channels: 255,
            output_channels: 255,
            number_of_clut_grid_points: 0,
            e_params: vec![],
            input_table: vec![],
            clut_values: vec![],
            output_table: vec![],
        }),
        Data::Lut16(Mft2 {
            input_channels: 255,
            output_channels: 255,
            number_of_clut_grid_points: 0,
            e_params: vec![],
            input_table_enteries: u16::MAX,
            output_table_enteries: u16::MAX,
            input_table: vec![],
            clut_values: vec![],
            output_table: vec![],
        }),
        Data::ParametricCurve(ParametricCurve {
            funtion_type: u16::MAX,
            vals: vec![],
        }),
        Data::OneDimenstionalCurves(OneDimensionalCurvesType {
            segments: u16::MAX,
            dimensional_curves: vec![vec![f32::NAN, f32::INFINITY], vec![]],
        }),
        Data::MultiLocalizedUnicode(MultiLocalizedUnicodeType {
            unicode_strings: vec![LocalizedUnicode {
                lang: "not a language".into(),
                country: "unknown".into(),
                unicode_string: "🦀".into(),
            }],
        }),
        Data::Descriptor(Descriptor {
            ascii_string: "not ASCII 🦀".into(),
            lang: "".into(),
            local_string: "".into(),
        }),
    ];
    for data in values {
        let mut value = profile();
        value.tags.insert("test".into(), data);
        std::panic::catch_unwind(|| exact_output_limits(&value))
            .expect("bounded formatter panicked");
    }
}

#[test]
fn oversized_public_strings_vectors_keys_and_dates_return_errors() {
    let values = [
        Data::ASCII("a".repeat(1_000_000)),
        Data::ASCII("\n".repeat(1_000_000)),
        Data::Binary(vec![0; 1_000_000]),
        Data::CrdInfo(vec!["a".repeat(1_000_000)]),
        Data::OneDimenstionalCurves(OneDimensionalCurvesType {
            segments: 0,
            dimensional_curves: vec![vec![]; 100_000],
        }),
    ];
    for data in values {
        let mut value = profile();
        value.tags.insert("test".into(), data);
        assert_eq!(
            decoded_print_with_limit(&value, 1024).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }
    let mut value = profile();
    value.create_date = "a".repeat(1_000_000);
    assert!(decoded_print_with_limit(&value, 1024).is_err());
    value.create_date.clear();
    value.tags.insert("a".repeat(1_000_000), Data::None);
    assert!(decoded_print_with_limit(&value, 1024).is_err());
}
