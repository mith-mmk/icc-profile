use super::error::TransformError;

pub(super) const D50: [f32; 3] = [0.9642, 1.0, 0.8249];
pub(super) const RGB: u32 = u32::from_be_bytes(*b"RGB ");
pub(super) const GRAY: u32 = u32::from_be_bytes(*b"GRAY");
pub(super) const XYZ: u32 = u32::from_be_bytes(*b"XYZ ");
pub(super) const LAB: u32 = u32::from_be_bytes(*b"Lab ");
pub(super) const CMYK: u32 = u32::from_be_bytes(*b"CMYK");

pub(super) fn checked_range(data: &[u8], offset: usize, size: usize) -> Result<(), TransformError> {
    let end = offset
        .checked_add(size)
        .ok_or(TransformError::ResourceLimit("offset arithmetic"))?;
    if end > data.len() {
        return Err(TransformError::InvalidProfile(
            "tag range is outside the profile",
        ));
    }
    Ok(())
}

pub(super) fn be_u16(data: &[u8], p: usize) -> Result<u16, TransformError> {
    checked_range(data, p, 2)?;
    Ok(u16::from_be_bytes([data[p], data[p + 1]]))
}

pub(super) fn be_u32(data: &[u8], p: usize) -> Result<u32, TransformError> {
    checked_range(data, p, 4)?;
    Ok(u32::from_be_bytes([
        data[p],
        data[p + 1],
        data[p + 2],
        data[p + 3],
    ]))
}

pub(super) fn be_i32(data: &[u8], p: usize) -> Result<i32, TransformError> {
    Ok(be_u32(data, p)? as i32)
}
