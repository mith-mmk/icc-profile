use icc_profile::{AlgorithmProfile, ClutInterpolation3D, Profile, Transform, TransformOptions};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

fn fixture() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/lab-pcs-rgb.icc")
        .to_str()
        .unwrap()
        .to_owned()
}
fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_icc-cli"))
        .args(args)
        .output()
        .unwrap()
}
fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).unwrap()
}
fn failure(output: Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("icc-cli: "));
}
fn convert(format: &str, samples: &str, extra: &[&str]) -> Output {
    let profile = fixture();
    let mut args = vec![
        "convert",
        "--source",
        &profile,
        "--destination",
        &profile,
        "--format",
        format,
        "--samples",
        samples,
    ];
    args.extend_from_slice(extra);
    cli(&args)
}

struct TempFile(PathBuf);
impl TempFile {
    fn new(data: &[u8]) -> Self {
        static ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "icc-cli-{}-{}.icc",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        file.write_all(data).unwrap();
        Self(path)
    }
    fn name(&self) -> &str {
        self.0.to_str().unwrap()
    }
}
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn help_inspect_validate() {
    assert!(success(cli(&["--help"])).contains("--interpolation"));
    let text = success(cli(&["inspect", &fixture()]));
    for field in [
        "size: 59444",
        "color-space: Rgb",
        "pcs: Lab",
        "version-bytes:",
        "device-class-bytes:",
    ] {
        assert!(text.contains(field), "{text}");
    }
    assert!(success(cli(&["validate", &fixture()])).contains("not universal ICC conformance"));
}

#[test]
fn numeric_outputs_match_checked_api_for_both_interpolations() {
    let bytes = fs::read(fixture()).unwrap();
    let profile = Profile::parse(&bytes).unwrap();
    for (name, interpolation) in [
        ("tetrahedral", ClutInterpolation3D::Tetrahedral),
        ("trilinear", ClutInterpolation3D::Trilinear),
    ] {
        let t = Transform::new(&profile, &profile, TransformOptions::default())
            .unwrap()
            .with_algorithm_profile(AlgorithmProfile::default().with_clut_3d(interpolation));
        let extra = ["--intent", "relative", "--interpolation", name];
        let text = success(convert("f32", "0,0.25,1 0.7,0.4,0.2", &extra));
        assert_eq!(text.lines().count(), 2);
        let actual: Vec<f32> = text
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        let mut expected = [0.0; 6];
        t.transform_f32(&[0.0, 0.25, 1.0, 0.7, 0.4, 0.2], &mut expected)
            .unwrap();
        assert_eq!(actual, expected);
        let text = success(convert("u8", "0,64,255", &extra));
        let actual: Vec<u8> = text
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        let mut expected = [0; 3];
        t.transform_u8(&[0, 64, 255], &mut expected).unwrap();
        assert_eq!(actual, expected);
        let text = success(convert("u16", "0,16384,65535", &extra));
        let actual: Vec<u16> = text
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        let mut expected = [0; 3];
        t.transform_u16(&[0, 16384, 65535], &mut expected).unwrap();
        assert_eq!(actual, expected);
    }
    assert_eq!(
        success(convert("u8", "1 2 3", &[])),
        success(convert(
            "u8",
            "1 2 3",
            &["--intent", "relative", "--interpolation", "tetrahedral"]
        ))
    );
}

#[test]
fn malformed_unsupported_and_missing_profiles() {
    let bad = TempFile::new(b"bad ICC");
    for command in ["inspect", "validate"] {
        failure(cli(&[command, bad.name()]), 4);
    }
    let mut bytes = fs::read(fixture()).unwrap();
    bytes[16..20].copy_from_slice(b"abcd");
    let unsupported = TempFile::new(&bytes);
    failure(cli(&["inspect", unsupported.name()]), 4);
    failure(
        cli(&[
            "convert",
            "--source",
            bad.name(),
            "--destination",
            &fixture(),
            "--format",
            "u8",
            "--samples",
            "0 0 0",
        ]),
        4,
    );
    failure(cli(&["inspect", &format!("{}.missing", bad.name())]), 3);
    failure(cli(&["inspect", env!("CARGO_MANIFEST_DIR")]), 3);
}

#[test]
fn bounded_profile_read() {
    let file = TempFile::new(b"");
    OpenOptions::new()
        .write(true)
        .open(&file.0)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    failure(cli(&["inspect", file.name()]), 4);
}

#[test]
fn invalid_arguments_and_sample_ranges() {
    for args in [
        vec![],
        vec!["unknown"],
        vec!["inspect"],
        vec!["inspect", "a", "b"],
        vec!["convert"],
        vec!["convert", "--source"],
        vec!["convert", "--output", "x"],
    ] {
        failure(cli(&args), 2);
    }
    for (format, samples) in [
        ("f32", "NaN 0 0"),
        ("f32", "inf 0 0"),
        ("f32", "-0.1 0 0"),
        ("f32", "1.1 0 0"),
        ("u8", "256 0 0"),
        ("u8", "-1 0 0"),
        ("u8", "1.5 0 0"),
        ("u16", "65536 0 0"),
        ("f64", "0 0 0"),
        ("u8", "0 0"),
        ("u8", ""),
        ("u8", "1,,2,3"),
    ] {
        failure(convert(format, samples, &[]), 2);
    }
    for extra in [
        ["--intent", "bogus"],
        ["--interpolation", "bogus"],
        ["--format", "u8"],
        ["--unknown", "x"],
    ] {
        failure(convert("u8", "0 0 0", &extra), 2);
    }
}

#[test]
fn unsupported_route_is_not_a_legacy_fallback() {
    let mut bytes = fs::read(fixture()).unwrap();
    let count = u32::from_be_bytes(bytes[128..132].try_into().unwrap()) as usize;
    for index in 0..count {
        let start = 132 + index * 12;
        if &bytes[start..start + 3] == b"A2B" {
            bytes[start..start + 3].copy_from_slice(b"zzz");
        }
    }
    let source = TempFile::new(&bytes);
    success(cli(&["inspect", source.name()]));
    failure(
        cli(&[
            "convert",
            "--source",
            source.name(),
            "--destination",
            &fixture(),
            "--format",
            "u8",
            "--samples",
            "0 0 0",
        ]),
        4,
    );
}

#[cfg(unix)]
#[test]
fn fifo_is_rejected_before_blocking_open() {
    use std::{
        process::Stdio,
        time::{Duration, Instant},
    };
    let fifo = TempFile::new(b"");
    fs::remove_file(&fifo.0).unwrap();
    assert!(Command::new("mkfifo")
        .arg(&fifo.0)
        .status()
        .unwrap()
        .success());
    let mut child = Command::new(env!("CARGO_BIN_EXE_icc-cli"))
        .args(["inspect", fifo.name()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("opening FIFO blocked instead of rejecting non-regular file");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    failure(child.wait_with_output().unwrap(), 3);
}
