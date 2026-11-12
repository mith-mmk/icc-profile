use std::{env, fs};
fn main() {
    let args: Vec<_> = env::args().collect();
    let intent: u32 = args.get(4).map(|s| s.parse().expect("intent must be 0..3")).unwrap_or(1);
    assert!(intent <= 3, "intent must be 0..3");
    let trilinear = match args.get(5).map(String::as_str).unwrap_or("tetrahedral") { "tetrahedral" => false, "trilinear" => true, _ => panic!("interpolation must be tetrahedral or trilinear") };
    let icc_intent = [icc_profile::RenderingIntent::Perceptual, icc_profile::RenderingIntent::RelativeColorimetric, icc_profile::RenderingIntent::Saturation, icc_profile::RenderingIntent::AbsoluteColorimetric][intent as usize];
    let source = fs::read(&args[1]).unwrap();
    let destination = fs::read(&args[2]).unwrap();
    let channels = if &source[16..20] == b"CMYK" { 4 } else { 3 };
    let output_channels = if &destination[16..20] == b"CMYK" { 4 } else { 3 };
    let n: usize = if channels == 4 { 9 } else { 17 };
    let mut input = Vec::new();
    for mut i in 0usize..n.pow(channels as u32) {
        let mut pixel = vec![0; channels];
        for c in (0..channels).rev() {
            pixel[c] = ((i % n) as f64 * 65535.0 / (n - 1) as f64).round() as u16;
            i /= n;
        }
        input.extend(pixel);
    }
    let len = input.len() / channels * output_channels;
    let mut output = vec![0u16; len];
    match (
        icc_profile::Profile::parse(&source),
        icc_profile::Profile::parse(&destination),
    ) {
        (Ok(s), Ok(d)) => {
            println!("icc parse=ok");
            match icc_profile::Transform::new(&s, &d, icc_profile::TransformOptions {rendering_intent:icc_intent, ..Default::default()}) {
                Ok(t) => match t.with_algorithm_profile(icc_profile::AlgorithmProfile::default().with_clut_3d(if trilinear { icc_profile::ClutInterpolation3D::Trilinear } else { icc_profile::ClutInterpolation3D::Tetrahedral })).transform_u16_atomic(&input, &mut output) {
                    Ok(()) => {
                        save(&args[3], "icc", &output);
                        println!("icc transform=ok");
                    }
                    Err(e) => println!("icc execute={e:?}"),
                },
                Err(e) => println!("icc compile={e:?}"),
            }
        }
        (s, d) => println!("icc parse source={:?} destination={:?}", s.err(), d.err()),
    }
    match (
        moxcms::ColorProfile::new_from_slice(&source),
        moxcms::ColorProfile::new_from_slice(&destination),
    ) {
        (Ok(s), Ok(d)) => {
            println!("mox parse=ok");
            let mut options = moxcms::TransformOptions::default();
            options.rendering_intent = [moxcms::RenderingIntent::Perceptual, moxcms::RenderingIntent::RelativeColorimetric, moxcms::RenderingIntent::Saturation, moxcms::RenderingIntent::AbsoluteColorimetric][intent as usize];
            options.allow_use_cicp_transfer = false;
            options.prefer_fixed_point = false;
            options.interpolation_method = moxcms::InterpolationMethod::Tetrahedral;
            println!("mox options={options:?}");
            match s.create_transform_16bit(
                if channels == 4 {
                    moxcms::Layout::Rgba
                } else {
                    moxcms::Layout::Rgb
                },
                &d,
                if output_channels == 4 { moxcms::Layout::Rgba } else { moxcms::Layout::Rgb },
                options,
            ) {
                Ok(t) => match t.transform(&input, &mut output) {
                    Ok(()) => {
                        save(&args[3], "mox", &output);
                        println!("mox transform=ok");
                    }
                    Err(e) => println!("mox execute={e:?}"),
                },
                Err(e) => println!("mox compile={e:?}"),
            }
        }
        (s, d) => println!("mox parse source={:?} destination={:?}", s.err(), d.err()),
    }
    println!("pixels={}", input.len() / channels);
}
fn save(prefix: &str, kind: &str, values: &[u16]) {
    fs::write(
        format!("{prefix}.{kind}.bin"),
        values
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    )
    .unwrap();
}
