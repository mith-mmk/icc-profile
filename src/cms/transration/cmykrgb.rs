
use crate::cms::transration::*;
use crate::Mft2;

pub fn cmyk_to_rgb(y:u8,m:u8,c:u8,k:u8) -> (u8,u8,u8) {
    let r = (c as i16 + k as i16 - 255).clamp(0, 255) as u8;
    let g = (m as i16 + k as i16 - 255).clamp(0, 255) as u8;
    let b = (y as i16 + k as i16 - 255).clamp(0, 255) as u8;
    (r,g,b)
}

pub fn cmyk_to_rgb_lut16(c:u8,m:u8,y:u8,k:u8,lut:&Mft2,wp:&WhitePoint) -> (u8,u8,u8) {
    let (l,a,b) = cmyk_to_lab_lut16(y,m,c,k,lut);
    let (x,y,z) = lab_to_xyz_wp(l,a,b,wp);
    let (r,g,b) = xyz_to_rgb(x as f64,y as f64,z as f64);

    (r,g,b)
}

pub fn cmyk_to_rgb_lut8(c:u8,m:u8,y:u8,k:u8,lut:&Mft1,wp:&WhitePoint) -> (u8,u8,u8) {
    let (l,a,b) = cmyk_to_lab_lut8(y,m,c,k,lut);
    let (x,y,z) = lab_to_xyz_wp(l,a,b,wp);
    let (r,g,b) = xyz_to_rgb(x as f64,y as f64,z as f64);

    (r,g,b)
}


pub fn cmyk_to_rgb_from_profile(c:u8,m:u8,y:u8,k:u8,decoded:&DecodedICCProfile) -> (u8,u8,u8) {
    if decoded.color_space == 0x434d594b {  // CMYK
        let lut = decoded.tags.get("A2B0");
        let wp = WhitePoint::from_profile(decoded);
        if let Some(lut) = lut {
            match lut {
                Data::Lut16(lut16) => {
                    return cmyk_to_rgb_lut16(c,m,y,k,lut16,&wp)
                },
                Data::Lut8(lut8) => {
                    return cmyk_to_rgb_lut8(c,m,y,k,lut8,&wp)

                },
                _ => {
                }
            }
        }
    }
    // not has profile
    cmyk_to_rgb(c, m, y, k)
}

/// RGB → CMYK 簡易版 (近傍探索による逆LUT参照)
/// 実装: RGB → XYZ → Lab → CMYK (B2A0 Lut逆参照)
pub fn rgb_to_cmyk_from_profile(r:u8,g:u8,b:u8,decoded:&DecodedICCProfile) -> (u8,u8,u8,u8) {
    if decoded.color_space == 0x434d594b {  // CMYK
        // RGB → XYZ (sRGB 行列)
        let (x, y, z) = rgb_to_xyz_from_f64(
            r as f64 / 255.0,
            g as f64 / 255.0,
            b as f64 / 255.0
        );
        
        // XYZ → Lab (D65 白点)
        let wp = WhitePoint::from_profile(decoded);
        let (l, a_val, b_val) = xyz_to_lab_wp(x, y, z, &wp);
        
        // Lab → CMYK (B2A0 Lut 逆参照 - 近傍探索)
        let lut = decoded.tags.get("B2A0");
        if let Some(lut) = lut {
            match lut {
                Data::Lut8(lut8) => {
                    // Lut8: input [0..256] per channel, 3D CLUT, output 4D
                    // 近傍探索: Lab を グリッド座標に正規化し、CLUT から最も近い CMYK を探す
                    let l_norm = (l / 100.0 * 255.0).clamp(0.0, 255.0) as u8;
                    let a_norm = ((a_val + 127.0) / 254.0 * 255.0).clamp(0.0, 255.0) as u8;
                    let b_norm = ((b_val + 127.0) / 254.0 * 255.0).clamp(0.0, 255.0) as u8;
                    
                    // Lut8の CLUT を直接参照（グリッドポイント）
                    // 簡易版: 入力値を 33 グリッド（標準解像度）にマップしてCLUT参照
                    if let Some((c, m, y_out, k)) = lab_to_cmyk_via_lut8(l_norm, a_norm, b_norm, lut8) {
                        return (c, m, y_out, k);
                    }
                },
                Data::Lut16(lut16) => {
                    let l_norm = (l / 100.0 * 65535.0).clamp(0.0, 65535.0) as u16;
                    let a_norm = ((a_val + 127.0) / 254.0 * 65535.0).clamp(0.0, 65535.0) as u16;
                    let b_norm = ((b_val + 127.0) / 254.0 * 65535.0).clamp(0.0, 65535.0) as u16;
                    
                    if let Some((c, m, y_out, k)) = lab_to_cmyk_via_lut16(l_norm, a_norm, b_norm, lut16) {
                        return (c, m, y_out, k);
                    }
                },
                _ => {}
            }
        }
    }
    // フォールバック: RGB を単純に CMYK に変換（逆色）
    let k = 255 - ((r as u16 + g as u16 + b as u16) / 3) as u8;
    let c = 255 - r;
    let m_out = 255 - g;
    let y_out = 255 - b;
    (c, m_out, y_out, k)
}

/// Lut8 経由で Lab → CMYK (近傍探索)
fn lab_to_cmyk_via_lut8(l: u8, a: u8, b: u8, lut: &Mft1) -> Option<(u8, u8, u8, u8)> {
    // グリッドポイント数（通常33×33×33）
    let grid_size = lut.number_of_clut_grid_points as usize;
    
    // L, A, B を グリッド座標 [0..grid_size-1] にマップ
    let l_grid = (l as f64 / 255.0 * (grid_size - 1) as f64).round() as usize;
    let a_grid = (a as f64 / 255.0 * (grid_size - 1) as f64).round() as usize;
    let b_grid = (b as f64 / 255.0 * (grid_size - 1) as f64).round() as usize;
    
    let l_grid = l_grid.min(grid_size - 1);
    let a_grid = a_grid.min(grid_size - 1);
    let b_grid = b_grid.min(grid_size - 1);
    
    // CLUT からインデックスを計算 (4出力の場合)
    // Index = ((l*size + a)*size + b) * 4
    let clut_idx = ((l_grid * grid_size + a_grid) * grid_size + b_grid) * 4;
    
    // CLUT が十分なサイズを持つか確認
    if clut_idx + 3 >= lut.clut_values.len() {
        return None;
    }
    
    // CLUT から CMYK を取得 (4バイト = C, M, Y, K)
    let c = lut.clut_values[clut_idx];
    let m = lut.clut_values[clut_idx + 1];
    let y_out = lut.clut_values[clut_idx + 2];
    let k = lut.clut_values[clut_idx + 3];
    
    Some((c, m, y_out, k))
}

/// Lut16 経由で Lab → CMYK (近傍探索)
fn lab_to_cmyk_via_lut16(l: u16, a: u16, b: u16, lut: &Mft2) -> Option<(u8, u8, u8, u8)> {
    // グリッドポイント数
    let grid_size = lut.number_of_clut_grid_points as usize;
    
    // L, A, B を グリッド座標にマップ
    let l_grid = (l as f64 / 65535.0 * (grid_size - 1) as f64).round() as usize;
    let a_grid = (a as f64 / 65535.0 * (grid_size - 1) as f64).round() as usize;
    let b_grid = (b as f64 / 65535.0 * (grid_size - 1) as f64).round() as usize;
    
    let l_grid = l_grid.min(grid_size - 1);
    let a_grid = a_grid.min(grid_size - 1);
    let b_grid = b_grid.min(grid_size - 1);
    
    // CLUT からインデックスを計算 (4出力の場合、各2バイト)
    let clut_idx = ((l_grid * grid_size + a_grid) * grid_size + b_grid) * 4;
    
    // CLUT が十分なサイズを持つか確認
    if clut_idx + 3 >= lut.clut_values.len() {
        return None;
    }
    
    // CLUT から CMYK を取得 (各値は u16 → u8 に変換)
    let c = (lut.clut_values[clut_idx] >> 8) as u8;
    let m = (lut.clut_values[clut_idx + 1] >> 8) as u8;
    let y_out = (lut.clut_values[clut_idx + 2] >> 8) as u8;
    let k = (lut.clut_values[clut_idx + 3] >> 8) as u8;
    
    Some((c, m, y_out, k))
}