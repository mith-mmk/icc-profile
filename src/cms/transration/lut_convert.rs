//! Legacy LUT helpers. Inputs retain their encoded byte domain and LUT16 outputs
//! retain the raw 0..65535 domain. Invalid public LUT payloads panic explicitly;
//! batch wrappers report InvalidInput instead. Three-channel helpers retain their
//! historical matrix stage (these APIs have no PCS metadata).
use crate::iccprofile::{ICCNumber, S15Fixed16Number};
use crate::{Mft1, Mft2};
use std::io::{Error, ErrorKind, Result};

pub(crate) fn batch_buffer<T>(
    len: usize,
    entries: usize,
    inputs: usize,
    outputs: usize,
) -> Result<Vec<T>> {
    let invalid = || Error::new(ErrorKind::InvalidInput, "Invalid batch length");
    let needed = entries.checked_mul(inputs).ok_or_else(invalid)?;
    let capacity = entries.checked_mul(outputs).ok_or_else(invalid)?;
    if len < needed {
        return Err(invalid());
    }
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(capacity).map_err(|_| invalid())?;
    Ok(buffer)
}

fn shape(
    channels: (u8, u8),
    grid: u8,
    matrix: usize,
    entries: (usize, usize),
    lengths: (usize, usize, usize),
    n: usize,
    m: usize,
) -> bool {
    channels.0 as usize == n
        && channels.1 as usize == m
        && grid >= 2
        && (n != 3 || matrix >= 9)
        && entries.0 >= 2
        && entries.1 >= 2
        && n.checked_mul(entries.0).is_some_and(|v| lengths.0 >= v)
        && (grid as usize)
            .checked_pow(n as u32)
            .and_then(|v| v.checked_mul(m))
            .is_some_and(|v| lengths.1 >= v)
        && m.checked_mul(entries.1).is_some_and(|v| lengths.2 >= v)
}

pub(crate) fn valid8(lut: &Mft1, n: usize, m: usize) -> bool {
    shape(
        (lut.input_channels, lut.output_channels),
        lut.number_of_clut_grid_points,
        lut.e_params.len(),
        (256, 256),
        (
            lut.input_table.len(),
            lut.clut_values.len(),
            lut.output_table.len(),
        ),
        n,
        m,
    )
}
pub(crate) fn valid16(lut: &Mft2, n: usize, m: usize) -> bool {
    shape(
        (lut.input_channels, lut.output_channels),
        lut.number_of_clut_grid_points,
        lut.e_params.len(),
        (
            lut.input_table_enteries as usize,
            lut.output_table_enteries as usize,
        ),
        (
            lut.input_table.len(),
            lut.clut_values.len(),
            lut.output_table.len(),
        ),
        n,
        m,
    )
}

pub(crate) fn check8(lut: &Mft1, n: usize, m: usize) -> Result<()> {
    if valid8(lut, n, m) {
        Ok(())
    } else {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "Invalid legacy LUT8 payload",
        ))
    }
}
pub(crate) fn check16(lut: &Mft2, n: usize, m: usize) -> Result<()> {
    if valid16(lut, n, m) {
        Ok(())
    } else {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "Invalid legacy LUT16 payload",
        ))
    }
}

fn table<T: Copy + Into<f64>>(values: &[T], coordinate: f64) -> f64 {
    let x = coordinate.clamp(0.0, 1.0) * (values.len() - 1) as f64;
    let lo = x as usize;
    let hi = (lo + 1).min(values.len() - 1);
    let fraction = x - lo as f64;
    values[lo].into() * (1.0 - fraction) + values[hi].into() * fraction
}

fn evaluate<T: Copy + Into<f64>, const N: usize, const M: usize>(
    input: [u8; N],
    matrix: &[S15Fixed16Number],
    grid: usize,
    tables: (&[T], &[T], &[T]),
    entries: (usize, usize),
    max: f64,
) -> [f64; M] {
    let mut coordinates = input.map(|v| v as f64 / 255.0);
    if N == 3 {
        let original = coordinates;
        for axis in 0..3 {
            coordinates[axis] = (0..3)
                .map(|i| matrix[axis * 3 + i].as_f64() * original[i])
                .sum::<f64>()
                .clamp(0.0, 1.0);
        }
    }
    let mut low = [0; N];
    let mut high = [0; N];
    let mut fraction = [0.0; N];
    for axis in 0..N {
        let offset = axis * entries.0;
        let x = table(&tables.0[offset..offset + entries.0], coordinates[axis]) / max
            * (grid - 1) as f64;
        low[axis] = x as usize;
        high[axis] = (low[axis] + 1).min(grid - 1);
        fraction[axis] = x - low[axis] as f64;
    }
    let mut output = [0.0; M];
    // ICC CLUT order: the first input varies slowest, outputs are interleaved.
    for corner in 0..(1 << N) {
        let mut index = 0;
        let mut weight = 1.0;
        for axis in 0..N {
            let upper = corner & (1 << axis) != 0;
            index = index * grid + if upper { high[axis] } else { low[axis] };
            weight *= if upper {
                fraction[axis]
            } else {
                1.0 - fraction[axis]
            };
        }
        for (channel, value) in output.iter_mut().enumerate() {
            *value += tables.1[index * M + channel].into() * weight;
        }
    }
    for (channel, value) in output.iter_mut().enumerate() {
        let offset = channel * entries.1;
        *value = table(&tables.2[offset..offset + entries.1], *value / max);
    }
    output
}

fn lut8<const N: usize, const M: usize>(input: [u8; N], lut: &Mft1) -> [u8; M] {
    check8(lut, N, M).expect("Invalid legacy LUT8 payload");
    evaluate::<_, N, M>(
        input,
        &lut.e_params,
        lut.number_of_clut_grid_points as usize,
        (&lut.input_table, &lut.clut_values, &lut.output_table),
        (256, 256),
        255.0,
    )
    .map(|v| v.round().clamp(0.0, 255.0) as u8)
}
fn lut16<const N: usize, const M: usize>(input: [u8; N], lut: &Mft2) -> [f64; M] {
    check16(lut, N, M).expect("Invalid legacy LUT16 payload");
    evaluate(
        input,
        &lut.e_params,
        lut.number_of_clut_grid_points as usize,
        (&lut.input_table, &lut.clut_values, &lut.output_table),
        (
            lut.input_table_enteries as usize,
            lut.output_table_enteries as usize,
        ),
        65535.0,
    )
}

/// Evaluate a 4-input, 3-output legacy LUT; panics on malformed payloads.
pub fn d4_to_d3_lut8(c0: u8, c1: u8, c2: u8, c3: u8, lut: &Mft1) -> (u8, u8, u8) {
    let v = lut8::<4, 3>([c0, c1, c2, c3], lut);
    (v[0], v[1], v[2])
}

/// Evaluate a 3-input, 4-output legacy LUT; panics on malformed payloads.
pub fn d3_to_d4_lut8(c0: u8, c1: u8, c2: u8, lut: &Mft1) -> (u8, u8, u8, u8) {
    let v = lut8::<3, 4>([c0, c1, c2], lut);
    (v[0], v[1], v[2], v[3])
}

/// Evaluate a 3-input, 3-output legacy LUT; panics on malformed payloads.
pub fn d3_to_d3_lut8(c0: u8, c1: u8, c2: u8, lut: &Mft1) -> (u8, u8, u8) {
    let v = lut8::<3, 3>([c0, c1, c2], lut);
    (v[0], v[1], v[2])
}

/// Evaluate a 3-input, 1-output legacy LUT; panics on malformed payloads.
pub fn d3_to_d1_lut8(c0: u8, c1: u8, c2: u8, lut: &Mft1) -> u8 {
    let v = lut8::<3, 1>([c0, c1, c2], lut);
    v[0]
}

/// Evaluate a 4-input, 3-output legacy LUT; panics on malformed payloads.
pub fn d4_to_d3_lut16(c0: u8, c1: u8, c2: u8, c3: u8, lut: &Mft2) -> (f64, f64, f64) {
    let v = lut16::<4, 3>([c0, c1, c2, c3], lut);
    (v[0], v[1], v[2])
}

/// Evaluate a 3-input, 4-output legacy LUT; panics on malformed payloads.
pub fn d3_to_d4_lut16(c0: u8, c1: u8, c2: u8, lut: &Mft2) -> (f64, f64, f64, f64) {
    let v = lut16::<3, 4>([c0, c1, c2], lut);
    (v[0], v[1], v[2], v[3])
}

/// Evaluate a 3-input, 3-output legacy LUT; panics on malformed payloads.
pub fn d3_to_d3_lut16(c0: u8, c1: u8, c2: u8, lut: &Mft2) -> (f64, f64, f64) {
    let v = lut16::<3, 3>([c0, c1, c2], lut);
    (v[0], v[1], v[2])
}

/// Evaluate a 3-input, 1-output legacy LUT; panics on malformed payloads.
pub fn d3_to_d1_lut16(c0: u8, c1: u8, c2: u8, lut: &Mft2) -> f64 {
    let v = lut16::<3, 1>([c0, c1, c2], lut);
    v[0]
}
