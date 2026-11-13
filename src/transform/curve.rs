use super::error::TransformError;
use super::limits::ParseLimits;
use super::reader::{be_i32, be_u16, be_u32, checked_range};

#[derive(Clone, Debug)]
pub(super) enum Curve {
    Identity,
    Gamma(f32),
    Table(Vec<f32>),
    Para {
        function: u16,
        values: Vec<f32>,
        direction: i8,
    },
}

impl Curve {
    pub(super) fn eval(&self, x: f32) -> f32 {
        self.eval_unclamped(x).clamp(0.0, 1.0)
    }

    pub(super) fn eval_unclamped(&self, x: f32) -> f32 {
        match self {
            Self::Identity => x,
            Self::Gamma(g) => x.powf(*g),
            Self::Table(t) => {
                if t.is_empty() {
                    return x;
                }
                if t.len() == 1 {
                    return t[0];
                }
                let p = x * (t.len() - 1) as f32;
                let i = if p < 0.0 {
                    0
                } else if p >= (t.len() - 1) as f32 {
                    t.len() - 2
                } else {
                    p.floor() as usize
                };
                let j = i + 1;
                t[i] + (t[j] - t[i]) * (p - i as f32)
            }
            Self::Para {
                function, values, ..
            } => eval_parametric(*function, values, x),
        }
    }
}

pub(super) fn inverse_curve(curve: &Curve, y: f32) -> f32 {
    match curve {
        Curve::Identity => y,
        Curve::Gamma(g) if *g != 0.0 => y.powf(1.0 / *g),
        Curve::Table(t) => {
            if t.len() < 2 {
                return y;
            }
            let increasing = t[0] <= *t.last().unwrap();
            let low = t[0].min(*t.last().unwrap());
            let high = t[0].max(*t.last().unwrap());
            let mut first_equal = None;
            let mut last_equal = None;
            for (index, value) in t.iter().enumerate() {
                if *value == y {
                    first_equal.get_or_insert(index);
                    last_equal = Some(index);
                }
            }
            if let (Some(first), Some(last)) = (first_equal, last_equal) {
                return (if last + 1 == t.len() { first } else { last }) as f32
                    / (t.len() - 1) as f32;
            }
            let mut i = 0;
            if increasing {
                while i + 1 < t.len() && t[i + 1] < y {
                    i += 1;
                }
            } else {
                while i + 1 < t.len() && t[i + 1] > y {
                    i += 1;
                }
            }
            if y <= low {
                i = 0;
            }
            if y >= high {
                i = t.len() - 2;
            }
            let d = t[i + 1] - t[i];
            (i as f32 + (y - t[i]) / d) / (t.len() - 1) as f32
        }
        Curve::Para { direction, .. } => {
            let direction = *direction as f32;
            let target = y * direction;
            // The endpoint policy determines which bound is needed before
            // searching. Computing the other bound cannot affect the result.
            let lower_bound = curve.eval_unclamped(1.0) * direction == target;
            let before = |value: f32| {
                let value = value * direction;
                value < target || (!lower_bound && value == target)
            };
            let (mut lo, mut hi, skipped) =
                inverse_search_seed(curve, y, &before).unwrap_or((0.0, 1.0, 0));
            for _ in skipped..32 {
                let mid = (lo + hi) * 0.5;
                // Once rounding repeats an endpoint, further iterations are
                // identical. Keep the original 32-step cap near zero, where
                // f32 spacing can remain finer than the search interval.
                let exhausted = mid == lo || mid == hi;
                if before(curve.eval_unclamped(mid)) {
                    lo = mid;
                } else {
                    hi = mid;
                }
                if exhausted {
                    break;
                }
            }
            if lower_bound {
                hi
            } else {
                lo
            }
        }
        Curve::Gamma(_) => y,
    }
}

// Use an analytic estimate only to locate an exact dyadic search cell. The
// endpoint predicates prove that the original bisection reaches this same
// cell after 20 steps. No approximate inverse is returned. If the estimate
// misses (including at a plateau or discontinuity), use the original domain.
fn inverse_search_seed(
    curve: &Curve,
    y: f32,
    before: &impl Fn(f32) -> bool,
) -> Option<(f32, f32, usize)> {
    let Curve::Para {
        function: 3,
        values,
        direction: 1,
    } = curve
    else {
        return None;
    };
    let (g, a, b, c, d) = (values[0], values[1], values[2], values[3], values[4]);
    // Prove monotonicity on the actual, unclamped branches. Restrict this
    // acceleration to the common positive type-3 (including sRGB) case.
    if !(g > 0.0
        && a > 0.0
        && c > 0.0
        && (0.0..=1.0).contains(&d)
        && a * d + b >= 0.0
        && c * d <= (a * d + b).powf(g))
    {
        return None;
    }
    let x = if y < curve.eval_unclamped(d) {
        y / c
    } else {
        (y.powf(1.0 / g) - b) / a
    };
    const CELLS: f32 = 1048576.0;
    if !(0.0..1.0).contains(&x) {
        return None;
    }
    let index = (x * CELLS) as u32;
    let lo = index as f32 / CELLS;
    let hi = (index + 1) as f32 / CELLS;
    if before(curve.eval_unclamped(lo)) && !before(curve.eval_unclamped(hi)) {
        Some((lo, hi, 20))
    } else {
        None
    }
}

pub(super) fn inverse_curve_checked(
    curve: &Curve,
    y: f32,
    clamp: bool,
) -> Result<f32, TransformError> {
    if !y.is_finite() {
        return Err(TransformError::NonFiniteInput);
    }
    let value = if clamp {
        y.clamp(0.0, 1.0)
    } else {
        let start = curve.eval_unclamped(0.0);
        let end = curve.eval_unclamped(1.0);
        if !start.is_finite() || !end.is_finite() {
            return Err(TransformError::InvalidProfile(
                "inverse curve has a non-finite endpoint",
            ));
        }
        let low = start.min(end);
        let high = start.max(end);
        if y < low || y > high {
            return Err(TransformError::MalformedProfile(
                "inverse curve target is outside its reachable domain".into(),
            ));
        }
        y
    };
    let result = inverse_curve(curve, value);
    if !result.is_finite() || (!clamp && !(0.0..=1.0).contains(&result)) {
        return Err(TransformError::MalformedProfile(
            "inverse curve produced a value outside its device domain".into(),
        ));
    }
    Ok(if clamp {
        result.clamp(0.0, 1.0)
    } else {
        result
    })
}

pub(super) fn parse_curve(data: &[u8], limits: ParseLimits) -> Result<Curve, TransformError> {
    if data.len() < 12 {
        return Err(TransformError::InvalidProfile("curve tag is truncated"));
    }
    match be_u32(data, 0)? {
        s if s == u32::from_be_bytes(*b"curv") => {
            let count = be_u32(data, 8)? as usize;
            if count > limits.max_curve_entries {
                return Err(TransformError::ResourceLimit("curve entries"));
            }
            if count == 0 {
                return Ok(Curve::Identity);
            }
            if count == 1 {
                let gamma = be_u16(data, 12)? as f32 / 256.0;
                if !gamma.is_finite() || gamma <= 0.0 {
                    return Err(TransformError::InvalidProfile("curve gamma"));
                }
                return Ok(Curve::Gamma(gamma));
            }
            checked_range(
                data,
                12,
                count
                    .checked_mul(2)
                    .ok_or(TransformError::ResourceLimit("curve arithmetic"))?,
            )?;
            let mut table = Vec::new();
            table
                .try_reserve_exact(count)
                .map_err(|_| TransformError::ResourceLimit("curve allocation"))?;
            for i in 0..count {
                table.push(be_u16(data, 12 + i * 2)? as f32 / 65535.0);
            }
            let increasing = table.windows(2).all(|pair| pair[0] <= pair[1]);
            let decreasing = table.windows(2).all(|pair| pair[0] >= pair[1]);
            if table.first() == table.last() || (!increasing && !decreasing) {
                return Err(TransformError::InvalidProfile(
                    "sampled curve must be monotonic and non-constant",
                ));
            }
            Ok(Curve::Table(table))
        }
        s if s == u32::from_be_bytes(*b"para") => {
            let function = be_u16(data, 8)?;
            let count = match function {
                0 => 1,
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 7,
                _ => {
                    return Err(TransformError::UnsupportedProfileFeature(
                        "parametric curve function",
                    ));
                }
            };
            checked_range(data, 12, count * 4)?;
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|_| TransformError::ResourceLimit("curve allocation"))?;
            for i in 0..count {
                values.push(be_i32(data, 12 + i * 4)? as f32 / 65536.0);
            }
            if values.iter().any(|v| !v.is_finite()) || values[0] <= 0.0 {
                return Err(TransformError::InvalidProfile(
                    "parametric curve parameters",
                ));
            }
            if matches!(function, 1 | 2) && values[1] == 0.0 {
                return Err(TransformError::InvalidProfile(
                    "parametric curve has zero coefficient",
                ));
            }
            if matches!(function, 3 | 4) && !(0.0..=1.0).contains(&values[4]) {
                return Err(TransformError::InvalidProfile(
                    "parametric curve threshold is outside the domain",
                ));
            }
            let direction = validate_parametric_curve_values(function, &values)?;
            let curve = Curve::Para {
                function,
                values: values.clone(),
                direction,
            };
            validate_parametric_domain_curve(&curve)?;
            for sample in [0.0, 1.0, values.get(4).copied().unwrap_or(0.0)] {
                if !curve.eval(sample).is_finite() {
                    return Err(TransformError::InvalidProfile(
                        "parametric curve has an invalid domain",
                    ));
                }
            }
            Ok(Curve::Para {
                function,
                values,
                direction,
            })
        }
        _ => Err(TransformError::UnsupportedProfileFeature("TRC type")),
    }
}

/// Parse a curve for a forward LUT stage.  Forward ICC curves are allowed to
/// contain flat or non-monotonic sampled data; monotonicity is only a
/// requirement when the curve is compiled as an inverse stage.
pub(super) fn parse_curve_forward(
    data: &[u8],
    limits: ParseLimits,
) -> Result<Curve, TransformError> {
    if data.len() < 12 {
        return Err(TransformError::InvalidProfile("curve tag is truncated"));
    }
    match be_u32(data, 0)? {
        s if s == u32::from_be_bytes(*b"curv") => {
            let count = be_u32(data, 8)? as usize;
            if count > limits.max_curve_entries {
                return Err(TransformError::ResourceLimit("curve entries"));
            }
            if count == 0 {
                return Ok(Curve::Identity);
            }
            if count == 1 {
                let gamma = be_u16(data, 12)? as f32 / 256.0;
                if !gamma.is_finite() || gamma <= 0.0 {
                    return Err(TransformError::InvalidProfile("curve gamma"));
                }
                return Ok(Curve::Gamma(gamma));
            }
            let bytes = count
                .checked_mul(2)
                .ok_or(TransformError::ResourceLimit("curve arithmetic"))?;
            checked_range(data, 12, bytes)?;
            let mut table = Vec::new();
            table
                .try_reserve_exact(count)
                .map_err(|_| TransformError::ResourceLimit("curve allocation"))?;
            for i in 0..count {
                table.push(be_u16(data, 12 + i * 2)? as f32 / 65535.0);
            }
            Ok(Curve::Table(table))
        }
        s if s == u32::from_be_bytes(*b"para") => {
            let function = be_u16(data, 8)?;
            let count: usize = match function {
                0 => 1,
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 7,
                _ => {
                    return Err(TransformError::UnsupportedProfileFeature(
                        "parametric curve function",
                    ));
                }
            };
            let bytes = count
                .checked_mul(4)
                .ok_or(TransformError::ResourceLimit("curve arithmetic"))?;
            checked_range(data, 12, bytes)?;
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|_| TransformError::ResourceLimit("curve allocation"))?;
            for i in 0..count {
                values.push(be_i32(data, 12 + i * 4)? as f32 / 65536.0);
            }
            if values.iter().any(|v| !v.is_finite()) || values[0] <= 0.0 {
                return Err(TransformError::InvalidProfile(
                    "parametric curve parameters",
                ));
            }
            if matches!(function, 1 | 2) && values[1] == 0.0 {
                return Err(TransformError::InvalidProfile(
                    "parametric curve has zero coefficient",
                ));
            }
            if matches!(function, 3 | 4) && !(0.0..=1.0).contains(&values[4]) {
                return Err(TransformError::InvalidProfile(
                    "parametric curve threshold is outside the domain",
                ));
            }
            let curve = Curve::Para {
                function,
                values,
                direction: 1,
            };
            validate_parametric_domain_curve(&curve)?;
            Ok(curve)
        }
        _ => Err(TransformError::UnsupportedProfileFeature("TRC type")),
    }
}

pub(super) fn validate_parametric_domain_curve(curve: &Curve) -> Result<(), TransformError> {
    let Curve::Para {
        function, values, ..
    } = curve
    else {
        return Ok(());
    };
    let samples = if matches!(*function, 3 | 4) {
        [0.0, values[4], 1.0]
    } else {
        [0.0, 0.5, 1.0]
    };
    if samples
        .iter()
        .any(|x| !curve.eval_unclamped(*x).is_finite())
    {
        return Err(TransformError::InvalidProfile(
            "parametric curve has non-finite domain",
        ));
    }
    let check_base = |x: f32| values[1] * x + values[2];
    // Functions 1 and 2 use the power branch only after its implicit
    // zero-crossing.  If that crossing is at or beyond one, the power branch
    // is not selected anywhere in the ICC input domain and its unused base
    // must not invalidate an otherwise finite forward curve.
    let power_domain_is_invalid = match *function {
        1 | 2 => {
            let crossing = -values[2] / values[1];
            crossing < 1.0
                && [crossing.clamp(0.0, 1.0), 1.0]
                    .iter()
                    .any(|x| check_base(*x) < 0.0)
        }
        // Functions 3 and 4 select the power branch at d.
        3 | 4 => [values[4], 1.0].iter().any(|x| check_base(*x) < 0.0),
        _ => false,
    };
    if power_domain_is_invalid {
        return Err(TransformError::InvalidProfile(
            "parametric curve has invalid power domain",
        ));
    }
    Ok(())
}

pub(super) fn validate_parametric_curve_values(
    function: u16,
    values: &[f32],
) -> Result<i8, TransformError> {
    let reject = || TransformError::UnsupportedProfileFeature("non-monotonic parametric curve");
    let sign = |value: f32| {
        if value > 0.0 {
            1
        } else if value < 0.0 {
            -1
        } else {
            0
        }
    };
    let mut direction = 0i8;
    let mut add = |value: f32| -> Result<(), TransformError> {
        let value = sign(value);
        if value != 0 {
            if direction != 0 && direction != value {
                return Err(reject());
            }
            direction = value;
        }
        Ok(())
    };
    match function {
        0 => add(eval_parametric(function, values, 1.0))?,
        1 | 2 => {
            let a = values[1];
            let b = values[2];
            let start = (-b / a).clamp(0.0, 1.0);
            if start >= 1.0 {
                return Err(reject());
            }
            let end = eval_parametric(function, values, 1.0);
            if !end.is_finite() {
                return Err(reject());
            }
            add(end - eval_parametric(function, values, start))?;
        }
        3 | 4 => {
            let a = values[1];
            let b = values[2];
            let c = values[3];
            let d = values[4];
            if a * d + b < 0.0 || a + b < 0.0 {
                return Err(reject());
            }
            if d > 0.0 {
                let low_start = eval_parametric(function, values, 0.0);
                let low_end = if function == 3 {
                    (c * d).clamp(0.0, 1.0)
                } else {
                    (c * d + values[6]).clamp(0.0, 1.0)
                };
                add(low_end - low_start)?;
                add(eval_parametric(function, values, d) - low_end)?;
            }
            if d < 1.0 {
                add(eval_parametric(function, values, 1.0) - eval_parametric(function, values, d))?;
            }
        }
        _ => {
            return Err(TransformError::UnsupportedProfileFeature(
                "parametric curve function",
            ));
        }
    }
    if direction == 0 {
        Err(reject())
    } else {
        Ok(direction)
    }
}

pub(super) fn eval_parametric(function: u16, values: &[f32], x: f32) -> f32 {
    match function {
        0 if !values.is_empty() => x.powf(values[0]),
        1 if values.len() >= 3 => {
            let (g, a, b) = (values[0], values[1], values[2]);
            if a != 0.0 && x >= -b / a {
                (a * x + b).powf(g)
            } else {
                0.0
            }
        }
        2 if values.len() >= 4 => {
            let (g, a, b, c) = (values[0], values[1], values[2], values[3]);
            if a != 0.0 && x >= -b / a {
                (a * x + b).powf(g) + c
            } else {
                c
            }
        }
        3 if values.len() >= 5 => {
            let (g, a, b, c, d) = (values[0], values[1], values[2], values[3], values[4]);
            if x >= d {
                (a * x + b).powf(g)
            } else {
                c * x
            }
        }
        4 if values.len() >= 7 => {
            let (g, a, b, c, d, e, f) = (
                values[0], values[1], values[2], values[3], values[4], values[5], values[6],
            );
            if x >= d {
                (a * x + b).powf(g) + e
            } else {
                c * x + f
            }
        }
        _ => x,
    }
}

#[cfg(test)]
mod inverse_search_tests {
    use super::*;

    // Frozen pre-optimization algorithm: compare bits, not a color tolerance.
    fn reference(curve: &Curve, y: f32, direction: i8) -> f32 {
        let direction = direction as f32;
        let target = y * direction;
        let (mut lower_lo, mut lower_hi) = (0.0, 1.0);
        let (mut upper_lo, mut upper_hi) = (0.0, 1.0);
        for _ in 0..32 {
            let mid = (lower_lo + lower_hi) * 0.5;
            if curve.eval_unclamped(mid) * direction < target {
                lower_lo = mid;
            } else {
                lower_hi = mid;
            }
            let mid = (upper_lo + upper_hi) * 0.5;
            if curve.eval_unclamped(mid) * direction <= target {
                upper_lo = mid;
            } else {
                upper_hi = mid;
            }
        }
        if curve.eval_unclamped(1.0) * direction == target {
            lower_hi
        } else {
            upper_lo
        }
    }

    #[test]
    fn seed_requires_a_proven_original_search_cell() {
        let curve = Curve::Para {
            function: 3,
            values: vec![2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045],
            direction: 1,
        };
        let y = 0.5;
        let before = |v| v <= y;
        let (lo, hi, depth) = inverse_search_seed(&curve, y, &before).unwrap();
        let (mut reference_lo, mut reference_hi) = (0.0, 1.0);
        for _ in 0..depth {
            let mid = (reference_lo + reference_hi) * 0.5;
            if before(curve.eval_unclamped(mid)) {
                reference_lo = mid;
            } else {
                reference_hi = mid;
            }
        }
        assert_eq!((lo, hi), (reference_lo, reference_hi));
        assert_eq!(depth, 20);
        assert!(inverse_search_seed(&curve, -1.0, &|v| v <= -1.0).is_none());
        let plateau = Curve::Para {
            function: 3,
            values: vec![1.0, 0.0, 0.5, 1.0, 0.5],
            direction: 1,
        };
        assert!(inverse_search_seed(&plateau, 0.5, &before).is_none());
    }

    #[test]
    fn parametric_search_matches_original_bits() {
        // All five functions, increasing/decreasing, lower/upper plateaus,
        // discontinuities, non-normalized ranges, and very small gamma.
        let cases: [(u16, Vec<f32>); 14] = [
            (0, vec![2.2]),
            (0, vec![0.01]),
            (0, vec![10.0]),
            (1, vec![2.0, 1.25, -0.25]),
            (2, vec![2.0, 1.25, -0.25, 0.125]),
            (
                3,
                vec![2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045],
            ),
            (3, vec![1.0, -1.0, 1.0, -1.0, 0.0]),
            (3, vec![1.0, 0.0, 0.5, 1.0, 0.5]),
            (3, vec![1.0, 1.0, 0.0, 0.0, 0.25]),
            (4, vec![2.0, 1.0, 0.0, 0.5, 0.5, 0.1, 0.0]),
            (4, vec![1.0, -1.0, 1.0, -1.0, 0.5, 0.0, 1.0]),
            (4, vec![1.0, 0.0, 0.5, -1.0, 0.5, 0.0, 1.0]),
            (4, vec![2.0, 1.0, 0.0, 0.0, 0.25, 0.5, 0.5]),
            (4, vec![1.0, 2.0, 0.0, 2.0, 0.5, 0.5, 0.0]),
        ];
        let cases: Vec<_> = cases
            .into_iter()
            .flat_map(|(function, values)| {
                let fixed = values
                    .iter()
                    .map(|v| (v * 65536.0).round() / 65536.0)
                    .collect();
                [(function, values), (function, fixed)]
            })
            .collect();
        for (function, values) in cases {
            let direction = validate_parametric_curve_values(function, &values).unwrap();
            let curve = Curve::Para {
                function,
                values,
                direction,
            };
            validate_parametric_domain_curve(&curve).unwrap();
            let mut targets = vec![
                0.0,
                -0.0,
                1.0,
                -1.0,
                2.0,
                f32::MIN_POSITIVE,
                f32::from_bits(1),
            ];
            for i in 0..=4096 {
                let x = i as f32 / 4096.0;
                targets.push(x);
                let y = curve.eval_unclamped(x);
                targets.extend([y, y.next_down(), y.next_up()]);
            }
            // Include fine dyadic cell boundaries, adjacent floats and
            // deterministic random targets, not just the coarse input grid.
            let mut state = 0x12345678u32;
            for _ in 0..8192 {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                let x = (state >> 12) as f32 / 1048576.0;
                let y = curve.eval_unclamped(x);
                targets.extend([y, y.next_down(), y.next_up(), f32::from_bits(state)]);
            }
            for y in targets.into_iter().filter(|y| y.is_finite()) {
                assert_eq!(
                    inverse_curve(&curve, y).to_bits(),
                    reference(&curve, y, direction).to_bits(),
                    "{curve:?}, y={y:?}"
                );
            }
        }
    }
}
