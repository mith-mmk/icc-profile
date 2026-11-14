use icc_profile::{Profile, RenderingIntent, Transform, TransformOptions};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 6 {
        return Err("usage: icc-colorsync-compare SOURCE DEST INPUT OUTPUT INTENT".into());
    }
    let sb = std::fs::read(&args[1])?;
    let db = std::fs::read(&args[2])?;
    let s = Profile::parse(&sb)?;
    let d = Profile::parse(&db)?;
    let intent = match args[5].as_str() {
        "0" => RenderingIntent::Perceptual,
        "1" => RenderingIntent::RelativeColorimetric,
        "2" => RenderingIntent::Saturation,
        "3" => RenderingIntent::AbsoluteColorimetric,
        _ => return Err("invalid intent".into()),
    };
    let input = std::fs::read(&args[3])?;
    if input.len() % 4 != 0 {
        return Err("input must be CMYK8".into());
    }
    let t = Transform::new(
        &s,
        &d,
        TransformOptions {
            rendering_intent: intent,
            black_point_compensation: false,
            ..Default::default()
        },
    )?;
    println!("{:?} -> {:?}", t.input_route_info(), t.output_route_info());
    let mut out = vec![0; input.len() / 4 * 3];
    t.transform_u8(&input, &mut out)?;
    std::fs::write(&args[4], out)?;
    Ok(())
}
