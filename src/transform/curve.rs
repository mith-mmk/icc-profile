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
            let mut lower_lo = 0.0;
            let mut lower_hi = 1.0;
            let mut upper_lo = 0.0;
            let mut upper_hi = 1.0;
            for _ in 0..32 {
                let lower_mid = (lower_lo + lower_hi) * 0.5;
                if curve.eval_unclamped(lower_mid) * direction < target {
                    lower_lo = lower_mid;
                } else {
                    lower_hi = lower_mid;
                }
                let upper_mid = (upper_lo + upper_hi) * 0.5;
                if curve.eval_unclamped(upper_mid) * direction <= target {
                    upper_lo = upper_mid;
                } else {
                    upper_hi = upper_mid;
                }
            }
            if curve.eval_unclamped(1.0) * direction == target {
                lower_hi
            } else {
                upper_lo
            }
        }
        Curve::Gamma(_) => y,
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
