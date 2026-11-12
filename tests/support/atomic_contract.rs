use icc_profile::Transform;

pub fn check_atomic_contract(transform: &Transform) {
    let ic = transform.input_channels();
    let oc = transform.output_channels();
    for pixels in [0, 1, 63, 64, 65, 129] {
        let input: Vec<f32> = (0..pixels * ic)
            .map(|i| 0.2 + (i % ic) as f32 * 0.1)
            .collect();
        let mut expected = vec![0.0; pixels * oc];
        transform.transform_f32(&input, &mut expected).unwrap();
        let mut actual = vec![-0.0; expected.len()];
        transform.transform_f32_atomic(&input, &mut actual).unwrap();
        assert_eq!(actual, expected);
        actual.fill(-0.0);
        transform
            .worker()
            .transform_f32_atomic(&input, &mut actual)
            .unwrap();
        assert_eq!(actual, expected);
        let input8: Vec<_> = input.iter().map(|v| (v * 255.0) as u8).collect();
        let input16: Vec<_> = input.iter().map(|v| (v * 65535.0) as u16).collect();
        let mut expected8 = vec![0; pixels * oc];
        let mut expected16 = vec![0; pixels * oc];
        transform.transform_u8(&input8, &mut expected8).unwrap();
        transform.transform_u16(&input16, &mut expected16).unwrap();
        let mut actual8 = vec![77; pixels * oc];
        let mut actual16 = vec![777; pixels * oc];
        transform
            .transform_u8_atomic(&input8, &mut actual8)
            .unwrap();
        transform
            .transform_u16_atomic(&input16, &mut actual16)
            .unwrap();
        assert_eq!(actual8, expected8);
        assert_eq!(actual16, expected16);
        actual8.fill(77);
        actual16.fill(777);
        transform
            .worker()
            .transform_u8_atomic(&input8, &mut actual8)
            .unwrap();
        transform
            .worker()
            .transform_u16_atomic(&input16, &mut actual16)
            .unwrap();
        assert_eq!(actual8, expected8);
        assert_eq!(actual16, expected16);
    }
    // Last channel of a pixel beyond two chunks must not publish any prefix.
    let mut input = vec![0.25; 129 * ic];
    *input.last_mut().unwrap() = f32::NAN;
    let sentinel = f32::from_bits(0x7fc01234);
    let mut output = vec![sentinel; 129 * oc];
    assert!(transform.transform_f32_atomic(&input, &mut output).is_err());
    assert!(output.iter().all(|v| v.to_bits() == sentinel.to_bits()));
    assert!(transform
        .worker()
        .transform_f32_atomic(&input, &mut output)
        .is_err());
    assert!(output.iter().all(|v| v.to_bits() == sentinel.to_bits()));
    // Output length is not interchangeable with input length for Gray/RGB.
    let mut output = vec![-0.0_f32; oc + 1];
    assert!(transform
        .transform_f32_atomic(&vec![0.25; ic], &mut output)
        .is_err());
    assert!(output.iter().all(|v| v.to_bits() == (-0.0_f32).to_bits()));
}

/// Manual paired timing, not a performance assertion. Run in release mode.
pub fn benchmark_atomic(transform: &Transform, label: &str) {
    use std::{hint::black_box, time::Instant};
    const PIXELS: usize = 65536;
    const REPEATS: usize = 4;
    let input = vec![0.4; PIXELS * transform.input_channels()];
    let mut output = vec![0.0; PIXELS * transform.output_channels()];
    let mut samples = [Vec::new(), Vec::new()];
    // Warm both paths. Alternate order to reduce drift bias.
    transform.transform_f32(&input, &mut output).unwrap();
    transform.transform_f32_atomic(&input, &mut output).unwrap();
    for round in 0..9 {
        for mode in [round % 2, 1 - round % 2] {
            let start = Instant::now();
            for _ in 0..REPEATS {
                if mode == 0 {
                    transform
                        .transform_f32(black_box(&input), black_box(&mut output))
                        .unwrap();
                } else {
                    transform
                        .transform_f32_atomic(black_box(&input), black_box(&mut output))
                        .unwrap();
                }
                black_box(&output);
            }
            samples[mode].push(start.elapsed().as_secs_f64() * 1e9 / (PIXELS * REPEATS) as f64);
        }
    }
    for sample in &mut samples {
        sample.sort_by(f64::total_cmp);
    }
    println!("{label}: f32 ns/pixel min/median/max {:.2}/{:.2}/{:.2}; atomic {:.2}/{:.2}/{:.2}; median ratio {:.3}",
        samples[0][0], samples[0][4], samples[0][8], samples[1][0], samples[1][4], samples[1][8], samples[1][4]/samples[0][4]);
}
