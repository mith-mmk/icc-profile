use icc_profile::{
    AlgorithmProfile, ClutInterpolation3D, Profile, RenderingIntent, Transform, TransformOptions,
};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::File,
    io::{self, Read, Write},
    path::Path,
    process::ExitCode,
};

const MAX_PROFILE: u64 = 64 * 1024 * 1024;
const MAX_ARGS: usize = 1024 * 1024;
const MAX_SAMPLES: usize = 65536;
const HELP: &str = "icc-cli inspect PROFILE\nicc-cli validate PROFILE\nicc-cli convert --source PROFILE --destination PROFILE --format f32|u8|u16 --samples \"VALUES\" [--intent relative|perceptual|saturation|absolute] [--interpolation tetrahedral|trilinear]\n\nVALUES: comma or ASCII whitespace separated device channels; f32 finite 0..1, u8 0..255, u16 0..65535.\nDefault intent: relative; interpolation: tetrahedral; BPC off; clipping on.\nOutput is stdout only, one converted pixel per line. No image decoding.\ninspect checks structure; validate checks supported profile semantics, not universal ICC conformance or transform viability.\nLimits: 64 MiB per regular profile file, 1 MiB argument text, 65536 sample components.\nExit: 0 success, 2 arguments/samples, 3 I/O, 4 profile/transform rejection.\n";

#[derive(Debug)]
struct Error(u8, String);
type Result<T> = std::result::Result<T, Error>;
fn usage(message: impl Into<String>) -> Error {
    Error(2, message.into())
}
fn io_error(e: io::Error) -> Error {
    Error(3, e.to_string())
}
fn cms_error(e: icc_profile::TransformError) -> Error {
    Error(4, e.to_string())
}

fn load(path: &Path, semantic: bool) -> Result<Profile> {
    // Reject ordinary FIFO/device paths before an open that might block.
    // The handle check below remains necessary; this is not race-proof.
    if !std::fs::metadata(path).map_err(io_error)?.is_file() {
        return Err(Error(3, "profile must be a regular file".into()));
    }
    let file = File::open(path).map_err(io_error)?;
    let metadata = file.metadata().map_err(io_error)?;
    if !metadata.is_file() {
        return Err(Error(3, "profile must be a regular file".into()));
    }
    if metadata.len() > MAX_PROFILE {
        return Err(Error(4, "profile exceeds 64 MiB".into()));
    }
    let mut data = Vec::new();
    file.take(MAX_PROFILE + 1)
        .read_to_end(&mut data)
        .map_err(io_error)?;
    if data.len() as u64 > MAX_PROFILE {
        return Err(Error(4, "profile exceeds 64 MiB".into()));
    }
    if semantic {
        Profile::from_bytes(&data)
    } else {
        Profile::parse(&data)
    }
    .map_err(cms_error)
}

fn options(args: &[OsString]) -> Result<BTreeMap<String, OsString>> {
    if !args.len().is_multiple_of(2) {
        return Err(usage("each convert option requires a value"));
    }
    let mut result = BTreeMap::new();
    for pair in args.as_chunks::<2>().0 {
        let key = pair[0]
            .to_str()
            .ok_or_else(|| usage("option must be UTF-8"))?;
        if ![
            "--source",
            "--destination",
            "--format",
            "--samples",
            "--intent",
            "--interpolation",
        ]
        .contains(&key)
        {
            return Err(usage(format!("unknown option: {key}")));
        }
        if result.insert(key.into(), pair[1].clone()).is_some() {
            return Err(usage(format!("duplicate option: {key}")));
        }
    }
    Ok(result)
}
fn required<'a>(options: &'a BTreeMap<String, OsString>, key: &str) -> Result<&'a OsString> {
    options
        .get(key)
        .ok_or_else(|| usage(format!("missing {key}")))
}
fn utf8(value: &OsString) -> Result<&str> {
    value
        .to_str()
        .ok_or_else(|| usage("option value must be UTF-8"))
}
fn tokens(samples: &str) -> Result<Vec<&str>> {
    // Empty comma fields are errors rather than silently dropped components.
    if samples.split(',').any(|part| part.trim().is_empty()) {
        return Err(usage("empty sample field"));
    }
    let values: Vec<_> = samples
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|s| !s.is_empty())
        .take(MAX_SAMPLES + 1)
        .collect();
    if values.is_empty() || values.len() > MAX_SAMPLES {
        return Err(usage("sample count must be 1..65536 components"));
    }
    Ok(values)
}
fn convert(args: &[OsString]) -> Result<String> {
    let opts = options(args)?;
    let source_path = required(&opts, "--source")?;
    let destination_path = required(&opts, "--destination")?;
    let format = utf8(required(&opts, "--format")?)?;
    if !["f32", "u8", "u16"].contains(&format) {
        return Err(usage("format must be f32, u8, or u16"));
    }
    let values = tokens(utf8(required(&opts, "--samples")?)?)?;
    let intent = match opts
        .get("--intent")
        .map(utf8)
        .transpose()?
        .unwrap_or("relative")
    {
        "relative" => RenderingIntent::RelativeColorimetric,
        "perceptual" => RenderingIntent::Perceptual,
        "saturation" => RenderingIntent::Saturation,
        "absolute" => RenderingIntent::AbsoluteColorimetric,
        _ => return Err(usage("unknown rendering intent")),
    };
    let interpolation = match opts
        .get("--interpolation")
        .map(utf8)
        .transpose()?
        .unwrap_or("tetrahedral")
    {
        "tetrahedral" => ClutInterpolation3D::Tetrahedral,
        "trilinear" => ClutInterpolation3D::Trilinear,
        _ => return Err(usage("unknown 3D interpolation")),
    };
    let source = load(Path::new(source_path), false)?;
    let destination = load(Path::new(destination_path), false)?;
    let transform = Transform::new(
        &source,
        &destination,
        TransformOptions {
            rendering_intent: intent,
            ..Default::default()
        },
    )
    .map_err(cms_error)?
    .with_algorithm_profile(AlgorithmProfile::default().with_clut_3d(interpolation));
    if !values.len().is_multiple_of(transform.input_channels()) {
        return Err(usage(format!(
            "sample count must be a multiple of {} source channels",
            transform.input_channels()
        )));
    }
    let length = values.len() / transform.input_channels() * transform.output_channels();
    macro_rules! run {
        ($ty:ty, $method:ident, $valid:expr) => {{
            let input: Vec<$ty> = values
                .iter()
                .map(|v| {
                    v.parse::<$ty>()
                        .map_err(|_| usage(format!("invalid {format} sample: {v}")))
                })
                .collect::<Result<_>>()?;
            if !input.iter().all($valid) {
                return Err(usage("f32 samples must be finite and in 0..1"));
            }
            let mut output = vec![0 as $ty; length];
            transform.$method(&input, &mut output).map_err(cms_error)?;
            let mut text = String::new();
            for pixel in output.chunks(transform.output_channels()) {
                for (i, value) in pixel.iter().enumerate() {
                    if i != 0 {
                        text.push(' ');
                    }
                    text.push_str(&value.to_string());
                }
                text.push('\n');
            }
            Ok(text)
        }};
    }
    match format {
        "f32" => run!(f32, transform_f32, |v: &f32| v.is_finite()
            && (0.0..=1.0).contains(v)),
        "u8" => run!(u8, transform_u8, |_: &u8| true),
        "u16" => run!(u16, transform_u16, |_: &u16| true),
        _ => unreachable!(),
    }
}
fn run(args: &[OsString]) -> Result<String> {
    if args
        .iter()
        .try_fold(0usize, |sum, arg| sum.checked_add(arg.len()))
        .is_none_or(|n| n > MAX_ARGS)
    {
        return Err(usage("arguments exceed 1 MiB"));
    }
    if args.len() == 1 && ["--help", "-h", "help"].iter().any(|s| args[0] == *s) {
        return Ok(HELP.into());
    }
    match args.first().and_then(|s| s.to_str()) {
        Some("inspect" | "validate") if args.len() == 2 => {
            let validate = args[0] == "validate";
            let profile = load(Path::new(&args[1]), validate)?;
            if validate {
                return Ok(
                    "valid: checked supported profile semantics (not universal ICC conformance)\n"
                        .into(),
                );
            }
            Ok(format!("size: {}\nversion-bytes: {:02x?}\ndevice-class-bytes: {:02x?}\ncolor-space: {:?}\npcs: {:?}\nheader-intent: {:?}\n", profile.size(), profile.raw_version(), profile.raw_device_class(), profile.color_space(), profile.pcs(), profile.rendering_intent()))
        }
        Some("convert") => convert(&args[1..]),
        _ => Err(usage("invalid command or arguments; use --help")),
    }
}
fn main() -> ExitCode {
    let result = run(&std::env::args_os().skip(1).collect::<Vec<_>>()).and_then(|output| {
        io::stdout()
            .lock()
            .write_all(output.as_bytes())
            .map_err(io_error)
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(Error(code, message)) => {
            let _ = writeln!(io::stderr().lock(), "icc-cli: {message}");
            ExitCode::from(code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_and_empty_samples() {
        for value in ["", " ", "1,,2", ",1", "1,"] {
            assert!(tokens(value).is_err());
        }
        assert_eq!(tokens("0, 0.5 1").unwrap(), ["0", "0.5", "1"]);
        assert!(tokens(&"0 ".repeat(MAX_SAMPLES + 1)).is_err());
        assert!(run(&[OsString::from("x".repeat(MAX_ARGS + 1))]).is_err());
    }
}
