#[test]
fn curves_cover_parametric_functions() {
    for (function, values) in [
        (0, vec![2.0]),
        (1, vec![2.0, 1.0, 0.0]),
        (2, vec![2.0, 1.0, 0.0, 0.1]),
        (3, vec![2.0, 1.0, 0.0, 0.5, 0.1]),
        (4, vec![2.0, 1.0, 0.0, 0.1, 0.0, 0.5, 0.0]),
    ] {
        let c = super::curve::Curve::Para {
            function,
            values,
            direction: 1,
        };
        assert!(c.eval(0.5).is_finite());
    }
}

#[test]
fn matrix_inverse_roundtrip() {
    let m = [[0.4, 0.2, 0.1], [0.1, 0.7, 0.2], [0.0, 0.1, 0.9]];
    let inv = super::profile::invert(m).unwrap();
    for row in 0..3 {
        for col in 0..3 {
            let got = (0..3).map(|k| m[row][k] * inv[k][col]).sum::<f32>();
            assert!((got - if row == col { 1.0 } else { 0.0 }).abs() < 1e-5);
        }
    }
}

#[test]
fn sampled_inverse_uses_complete_equal_runs() {
    let curve = super::curve::Curve::Table(vec![0.0, 32768.0 / 65535.0, 32768.0 / 65535.0, 1.0]);
    assert!((super::curve::inverse_curve(&curve, 32768.0 / 65535.0) - 2.0 / 3.0).abs() < 1e-7);
    let endpoint = super::curve::Curve::Table(vec![0.0, 1.0, 1.0]);
    assert!((super::curve::inverse_curve(&endpoint, 1.0) - 0.5).abs() < 1e-7);
}

#[test]
fn parametric_inverse_preserves_near_endpoint_plateau() {
    let curve = super::curve::Curve::Para {
        function: 1,
        values: vec![1.0, 32767.0, -32766.984375],
        direction: 1,
    };
    let values = match &curve {
        super::curve::Curve::Para { values, .. } => values,
        _ => unreachable!(),
    };
    let expected = -values[2] / values[1];
    assert!((super::curve::inverse_curve(&curve, 0.0) - expected).abs() < 1e-6);
}
