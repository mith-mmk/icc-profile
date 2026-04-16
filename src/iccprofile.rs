//! ICC Profile reader

use std::collections::HashMap;
use std::io::{Error,ErrorKind};
use std::io::Result;
use bin_rs::io::*;
use bin_rs::Endian;
use crate::iccprofile::Data::*;
use crate::utils::bound_check;

pub fn icc_profile_decode(data :&Vec<u8>) -> Result<DecodedICCProfile> {
    let icc_profile = ICCProfile::new(data)?;

    let mut decoded: HashMap<String,Data> = HashMap::new();
    let header_size = 128;
    let mut ptr = header_size;
    let tags = read_u32_be(&icc_profile.data,ptr);
    ptr +=  4;
    bound_check(&icc_profile.data,ptr,tags as usize * 12)?;
    for _ in 0..tags {
        let tag_name = read_string(&icc_profile.data,ptr,4);
        ptr +=  4;
        let tag_offset = read_u32_be(&icc_profile.data,ptr) as usize;
        ptr +=  4;
        let tag_length = read_u32_be(&icc_profile.data,ptr) as usize;
        ptr +=  4;
        let (_,val) = Data::parse(&icc_profile.data[tag_offset..],tag_length,icc_profile.version)?;
        decoded.insert(tag_name,val);
    }
    Ok(DecodedICCProfile {
        length : icc_profile.length, 
        cmmid : icc_profile.cmmid,
        version :icc_profile.version,
        device_class :icc_profile.device_class,
        color_space : icc_profile.color_space,
        pcs : icc_profile.pcs,
        create_date: icc_profile.create_date.clone(),
        magicnumber_ascp: icc_profile.magicnumber_ascp,
        platform: icc_profile.platform,
        flags: icc_profile.flags,
        manufacturer: icc_profile.manufacturer,
        model: icc_profile.model,
        attributes: icc_profile.attributes,
        rendering_intent: icc_profile.rendering_intent,
        illuminate :icc_profile.illuminate,
        creator: icc_profile.creator,
        profile_id: icc_profile.profile_id,
        tags: decoded,
    })
}

#[derive(Debug)]
pub struct DecodedICCProfile {
    pub length : u32,
    pub cmmid : u32,
    pub version :u32,
    pub device_class :u32,
    pub color_space : u32,
    pub pcs : u32,
    pub create_date: String,
    pub magicnumber_ascp: u32,
    pub platform: u32,
    pub flags: u32,
    pub manufacturer: u32,
    pub model: u32,
    pub attributes: u64,
    pub rendering_intent: u32,
    pub illuminate :XYZNumber,
    pub creator: u32,
    pub profile_id: u128,
    pub tags: HashMap<String,Data>,
}

impl DecodedICCProfile {
    pub fn new(buffer :&Vec<u8>) -> Result<Self> {
        icc_profile_decode(buffer)
    }

    pub fn to_string(&self) -> Result<String> {
        crate::utils::decoded_print(self,0)
    }

    pub fn to_string_with_verbose(&self,verbose:usize) -> Result<String> {
        crate::utils::decoded_print(self,verbose)
    }

}

#[derive(Debug)]
pub struct ICCProfile {
    pub length : u32,
    pub cmmid : u32,
    pub version :u32,
    pub device_class :u32,
    pub color_space : u32,
    pub pcs : u32,
    pub create_date: String,
    pub magicnumber_ascp: u32,
    pub platform: u32,
    pub flags: u32,
    pub manufacturer: u32,
    pub model: u32,
    pub attributes: u64,
    pub rendering_intent: u32,
    pub illuminate :XYZNumber,
    pub creator: u32,
    pub profile_id: u128,
    pub reserved :Vec<u8>,  // 28byte,
    pub data: Vec<u8>   // raw data
}

impl ICCProfile {    
    pub fn new(buffer :&Vec<u8>) -> Result<Self> {
        if buffer.len() < 128 {
            return Err(Error::new(ErrorKind::Other,"ICCProfile data shotage"))
        }
        let mut ptr = 0;
        let length = read_u32_be(&buffer,ptr);
        ptr += 4;
        let cmmid = read_u32_be(&buffer,ptr);
        ptr += 4;
        let version = read_u32_be(&buffer,ptr);
        ptr += 4;
        let device_class = read_u32_be(&buffer,ptr);
        ptr += 4;
        let color_space = read_u32_be(&buffer,ptr);
        ptr += 4;
        let pcs = read_u32_be(&buffer,ptr);
        ptr += 4;
        let year = read_u16_be(&buffer,ptr);
        ptr += 2;
        let month = read_u16_be(&buffer,ptr);
        ptr += 2;
        let day = read_u16_be(&buffer,ptr);
        ptr += 2;
        let hour = read_u16_be(&buffer,ptr);
        ptr += 2;
        let minute = read_u16_be(&buffer,ptr);
        ptr += 2;
        let second = read_u16_be(&buffer,ptr);
        ptr += 2;
        let magicnumber_ascp = read_u32_be(&buffer,ptr);
        ptr += 4;
        let platform = read_u32_be(&buffer,ptr);
        ptr += 4;
        let flags = read_u32_be(&buffer,ptr);
        ptr += 4;
        let manufacturer = read_u32_be(&buffer,ptr);
        ptr += 4;
        let model = read_u32_be(&buffer,ptr);
        ptr += 4;
        let attributes = read_u64_be(&buffer,ptr);
        ptr += 8;
        let rendering_intent = read_u32_be(&buffer,ptr);
        ptr += 4;
        let x = S15Fixed16Number{
            integer: read_i16_be(buffer, ptr),
            decimal: read_u16_be(buffer, ptr+2)
        };    
        ptr += 4;
        let y = S15Fixed16Number{
            integer: read_i16_be(buffer, ptr),
            decimal: read_u16_be(buffer, ptr+2)
        };        
        ptr += 4;
        let z = S15Fixed16Number{
            integer: read_i16_be(buffer, ptr),
            decimal: read_u16_be(buffer, ptr+2)
        };        
        ptr += 4;
        let illuminate = XYZNumber{x,y,z};

        let creator = read_u32_be(&buffer,ptr);
        ptr += 4;
        let profile_id = read_u128_be(&buffer, ptr);
//        ptr += 28;  // padding data

        let create_date = format!("{:>4}/{:>2}/{:>2} {:>02}:{:>02}:{:>02}",
            year,month,day,hour,minute,second);
        Ok(Self {
            length: length,
            cmmid : cmmid,
            version: version,
            device_class: device_class,
            color_space: color_space,
            pcs: pcs,
            create_date: create_date.clone(),
            magicnumber_ascp: magicnumber_ascp,
            platform: platform,
            flags: flags,
            manufacturer: manufacturer,
            model: model,
            attributes: attributes,
            rendering_intent: rendering_intent,
            illuminate: illuminate,
            creator: creator,
            profile_id: profile_id,
            reserved: Vec::new(),
            data : buffer.to_vec(),
        })
    }
}


pub trait ICCNumber {
    fn as_f32(&self) -> f32;
    fn as_f64(&self) -> f64;
    fn int(&self) -> i32;
    fn decimal(&self) -> u32;
}

#[derive(Debug)]
pub struct S15Fixed16Number {
    integer: i16,
    decimal: u16,
}

impl S15Fixed16Number {
    pub fn new(val: f64) -> Self {
        let integer = val.floor() as i16;
        let decimal = ((val - val.floor()) * u16::MAX as f64) as u16;
        S15Fixed16Number { integer, decimal }
    }
}

impl ICCNumber for S15Fixed16Number {
    fn as_f32(&self) -> f32 { self.integer as f32 + self.decimal as f32 / u16::MAX as f32 }
    fn as_f64(&self) -> f64 { self.integer as f64 + self.decimal as f64 / u16::MAX as f64 }
    fn int(&self) -> i32 { self.integer as i32 }
    fn decimal(&self) -> u32 { self.decimal as u32}
}

#[derive(Debug)]
pub struct U16Fixed16Number {
    integer:u16,
    decimal:u16,
}

impl ICCNumber for U16Fixed16Number {
    fn as_f32(&self) -> f32 { self.integer as f32 + self.decimal as f32 / u16::MAX as f32 }
    fn as_f64(&self) -> f64 { self.integer as f64 + self.decimal as f64 / u16::MAX as f64 }
    fn int(&self) -> i32 { self.integer as i32 }
    fn decimal(&self) -> u32 { self.decimal as u32}
}

#[derive(Debug)]
pub struct U1Fixed15Number {
    decimal:u16,
}

impl ICCNumber for U1Fixed15Number {
    fn as_f32(&self) -> f32 { self.decimal as f32 / i16::MAX as f32 }
    fn as_f64(&self) -> f64 { self.decimal as f64 / i16::MAX as f64 }
    fn int(&self) -> i32 { 0 }
    fn decimal(&self) -> u32 { self.decimal as u32}
}

#[derive(Debug)]
pub struct U8Fixed8Number {
    integer:u8,
    decimal:u8,
}

impl ICCNumber for U8Fixed8Number {
    fn as_f32(&self) -> f32 { self.integer as f32 + self.decimal as f32 / u8::MAX as f32 }
    fn as_f64(&self) -> f64 { self.integer as f64 + self.decimal as f64 / u8::MAX as f64  }
    fn int(&self) -> i32 { self.integer as i32 }
    fn decimal(&self) -> u32 { self.decimal as u32}
}

#[derive(Debug)]
pub struct XYZNumber {
    pub x:S15Fixed16Number,
    pub y:S15Fixed16Number,
    pub z:S15Fixed16Number
}

impl XYZNumber {
    pub fn to_string(&self) -> String {
        format!("X:{} Y:{} Z:{} ",self.x.as_f32(),self.y.as_f32(),self.z.as_f32())
    }
}

/// Mft1 is Lut8 type
#[derive(Debug)]
pub struct Mft1 {
    pub input_channels :u8,
    pub output_channels:u8,
    pub number_of_clut_grid_points:u8,
    pub e_params:Vec<S15Fixed16Number>,
    pub input_table: Vec<u8>,
    pub clut_values: Vec<u8>,
    pub output_table: Vec<u8>,
}

/// Mft2 is Lut16 type
#[derive(Debug)]
pub struct Mft2 {
    pub input_channels :u8,
    pub output_channels:u8,
    pub number_of_clut_grid_points:u8,
    pub e_params:Vec<S15Fixed16Number>,
    pub input_table_enteries: u16,
    pub output_table_enteries: u16,
    pub input_table: Vec<u16>,
    pub clut_values: Vec<u16>,
    pub output_table: Vec<u16>, 
}

/// enum Curve is for LutAtoB type and LutBtoA type
#[derive(Debug)]
pub enum Curve {
    ParametricCurve(ParametricCurve),
    Curve(Vec<u16>),
}

impl Curve {
    pub fn as_size(&self) -> usize {
        match self {
            Curve::Curve(curve) => {
                curve.len() * 2 + 12
            },
            Curve::ParametricCurve(curve) => {
                curve.as_size() + 8
            }
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Curve::Curve(curve) => {
                format!("Curve Type\n{:?}\n",curve)
            },
            Curve::ParametricCurve(curve) => {
                let mut str = "Parametic Curve Type\n".to_string();
                str += &match curve.funtion_type {
                    0x000 => {"function Y = X**ganma\n"},
                    0x001 => {"function Y = (aX+b)**ganma (X >= -b/a), Y = 0 (X < -b/a)\n"},
                    0x002 => {"function Y = (aX+b)**ganma + c(X >= -b/a), Y = c (X < -b/a)\n"},
                    0x003 => {"function Y = (aX+b)**ganma (X >= d), Y = cX (X < d)\n"},
                    0x004 => {"function Y = (aX+b)**ganma + e(X >= d), Y = cX + f (X < d)\n"},
                    _ => {"function Unknown"},
                }.to_string();
                for f in &curve.vals {
                    str += &f.as_f32().to_string();
                    str += " ";
                }
                str += "\n";
                str
           }
        }
    }
}

/// enum Clut is for LutAtoB type and LutBtoA type
#[derive(Debug)]
pub enum Clut {
    UInt8(Vec<u8>),
    UInt16(Vec<u16>),
}


#[derive(Debug)]
pub struct MClut {
    grid_points: Vec<u8>,   // max 16
    precision: u8,
    clut_data: Clut,
}

impl MClut {
    pub fn len(&self) -> usize {
        match &self.clut_data {
            Clut::UInt16(clut) => {
                clut.len()
            },
            Clut::UInt8(clut) => {
                clut.len()
            }
        }
    }
}


/// Mba is LutBtoA type
#[derive(Debug)]
pub struct Mba {
    pub input_channels :u8,
    pub output_channels:u8,
    pub b_curves: Vec<Curve>,
    pub matrix: Vec<S15Fixed16Number>,
    pub m_curves: Vec<Curve>,
    pub clut: MClut,
    pub a_curves: Vec<Curve>,
}

/// Mab is LutAtoB type
#[derive(Debug)]
pub struct Mab {
    pub input_channels :u8,
    pub output_channels:u8,
    pub b_curves: Vec<Curve>,
    pub matrix: Vec<S15Fixed16Number>,
    pub m_curves: Vec<Curve>,
    pub clut: MClut,
    pub a_curves: Vec<Curve>,
}

#[derive(Debug)]
pub struct ResponseCurveSet16 {
    pub number_of_channels: u16,
    pub count_of_measirement_types: u16,
    pub response_curve_structures: Vec<CurveStructure>,
}

#[derive(Debug)]
pub struct Response16Number {
    pub encoding_the_interval: u16,
    reserved: u16,
    pub measurement_value:S15Fixed16Number
}

#[derive(Debug)]
pub struct CurveStructure {
    pub signature: u32,
    pub for_each_channel:Vec<u32>,
    pub patch_with_the_maximum_colorant_value:Vec<XYZNumber>,
    pub response_arrays:Vec<Response16Number>,
}

#[derive(Debug)]
pub struct ParametricCurve {
    pub funtion_type:u16,
    pub vals:Vec<S15Fixed16Number>,
}

impl ParametricCurve {
    pub fn as_size(&self) -> usize {
        self.vals.len() * 4 + 4
    }
}

#[derive(Debug)]
pub struct FormulaCurve {
    pub funtion_type:u16,
    pub vals:Vec<f32>,
}

impl FormulaCurve {
    pub fn as_size(&self) -> usize {
        self.vals.len() * 4 + 2
    }
}

#[derive(Debug)]
pub struct DateTime{
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minutes: u32,
    pub second: u32,
}

impl DateTime{
    fn as_string(&self) -> String{
        format!("{:4}-{:02}-{:02} {:02}:{:02}:{:02}",
        self.year,self.month,self.day,self.hour,self.minutes,self.second)
    }
}

#[derive(Debug)]
pub struct ColorNameWithData {
    pub root_color_name: String,
    pub pcs: [u16;3],
    pub coordinate: Vec<u16>,
}


#[derive(Debug)]
pub struct NamedColor2Type {
    pub vendor_specific_flag: u32,
    pub prefix_color_name:String,
    pub suffix_color_name:String,
    pub entries:Vec<ColorNameWithData>,
}

#[derive(Debug)]
pub struct ViewingConditions {
    pub illuminant: XYZNumber,
    pub surround: XYZNumber,
    pub illuminant_type:u32,
}

#[derive(Debug)]
pub struct ChromaticityType {
    pub device_channels: u16,
    pub encoded_value: u16,
    pub cie_xy_coordinate_values:Vec<(U16Fixed16Number,U16Fixed16Number)>,
}  

#[derive(Debug)]
pub struct MeasurementType {
    pub standard_observer:u32,
    pub measurement_backing:XYZNumber,
    pub measurement_geometry:u32,
    pub measurement_flare:U16Fixed16Number,
    pub standard_illuminant: u32,
}

#[derive(Debug)]
pub struct ColorantTableType {
    pub colorant_name:String,
    pub pcs_values:Vec<[u16;3]>,
}

#[derive(Debug)]
pub struct PositionNumber {
    pub offset:u32,
    pub size:u32,
}

#[derive(Debug)]
pub struct MultiProcessElementsType {
    pub input_channels: u16,
    pub output_channels: u16,
    pub process_element_positions: Vec<PositionNumber>,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct CurveSetType {
    pub input_channels: u16,
    pub output_channels: u16,
    pub curve_positions: Vec<PositionNumber>,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct OneDimensionalCurvesType {
    pub segments: u16,
    pub dimensional_curves: Vec<Vec<f32>>,
}

#[derive(Debug)]
pub struct MatrixElement {
    pub input_channels: u16,
    pub output_channels: u16,
    pub matrix: Vec<f32>,
}

#[derive(Debug)]
pub struct Descriptor {
    pub ascii_string: String,
    pub lang: String,
    pub local_string: String,
}

impl Descriptor {
    pub fn to_string(&self) -> String {
        self.ascii_string.to_string()
    }
}

#[derive(Debug)]
pub struct LocalizedUnicode {
    pub lang: String,
    pub country: String,
    pub unicode_string : String,
}

impl LocalizedUnicode {
    pub fn as_string(&self) -> String {
        self.unicode_string.to_string()
    }
}

#[derive(Debug)]
pub struct MultiLocalizedUnicodeType {
    pub unicode_strings : Vec<LocalizedUnicode>,
}

impl MultiLocalizedUnicodeType {
    pub fn as_string(&self) -> String {
        if self.unicode_strings.len() > 0 {
            self.unicode_strings[0].unicode_string.to_string()
        } else {
            "".to_string()
        }
    }

    pub fn from(ascii: String) -> Self {
        let unicode_string = LocalizedUnicode{
            lang: "C".to_string(),
            country: "".to_string(),
            unicode_string: ascii,
        };
        let unicode_strings = vec![unicode_string];
        Self {
            unicode_strings
        }
    }

    pub fn list_string(&self) -> String {
        let mut str = "".to_string();
        for unicode_string in &self.unicode_strings{
            str += &format!("[{}_{}] {}\n",unicode_string.lang,unicode_string.country,unicode_string.as_string());
        }
        str.to_string()
    }
}

#[derive(Debug)]
pub struct ProfileDescription {
    pub device_manufacturer:u32,
    pub device_model:u32,
    pub device_attributes:String,
    pub technology_information:u32,
    pub description_device_manufacturer: MultiLocalizedUnicodeType,
    pub description_device_model: MultiLocalizedUnicodeType,
}

#[derive(Debug)]
pub enum Data {
    Raw(String,Vec<u8>), // no decodee
    // General ICC 3.2
    DataTimeNumber(DateTime),
    U16Fixed16Number(U16Fixed16Number),
    U8Fixed8Number(U8Fixed8Number),
    UInt16Number(u16),
    UInt32Number(u32),
    UInt64Number(u64),
    UInt8Number(u8),
    XYZNumber(XYZNumber),

    ASCII(String),
    Binary(Vec<u8>),


    Float32Number(f32),
    PositionNumber(PositionNumber),
    S15Fixed16Number(S15Fixed16Number),
    S15Fixed16NumberArray(Vec<S15Fixed16Number>),
    U16Fixed16NumberArray(Vec<U16Fixed16Number>),
    Response16Number(Response16Number),
    U1Fixed15Number(U1Fixed15Number),
    UInt16NumberArray(Vec<u16>),
    UInt32NumberArray(Vec<u32>),
    UInt64NumberArray(Vec<u64>),
    UInt8NumberArray(Vec<u8>),
    XYZNumberArray(Vec<XYZNumber>),

    // Type ICC 3.2
//    OldNamedColor(NamedColorType), // obsolute
    Curve(Vec<u16>),
    ParametricCurve(ParametricCurve),
    Lut8(Mft1),
    Lut16(Mft2),
    Chromaticity(ChromaticityType),
    MultiLocalizedUnicode(MultiLocalizedUnicodeType),
    ViewConditions(ViewingConditions),
    Measurement(MeasurementType),
    CurveSet(CurveSetType),
    ProfileDescription(Vec<ProfileDescription>),

    // Type ICC 4.0
    FormulaCurve(FormulaCurve),
    SampledCurve(Vec<f32>),
    NamedColor(NamedColor2Type),
    Descriptor(Descriptor),
    LutAtoB(Mab),
    LutBtoA(Mba),
    ResponseCurveSet16(ResponseCurveSet16),
    CrdInfo(Vec<String>),
    ColorantTable(ColorantTableType),
    MultiProcessElements(MultiProcessElementsType),
    OneDimenstionalCurves(OneDimensionalCurvesType),
    MatrixElement(MatrixElement),
    None,
}

fn illuminant_type_string(measurement_illuminate: u32) -> String {
    let string = match measurement_illuminate {
        0x00000001 => {"D50"},
        0x00000002 => {"D65"},
        0x00000003 => {"D93"},
        0x00000004 => {"F2"},
        0x00000005 => {"D55"},
        0x00000006 => {"A"},
        0x00000007 => {"Equi-Power (E)"},
        0x00000008 => {"F8\n"},
        _ => {"unknown"},
    };
    string.to_string()
}

impl Data {
    pub fn parse(data: &[u8],length:usize,version:u32) -> Result<(String,Data)> {
        let data_type = Self::read_data_type(data,0)?;
        Ok((data_type.clone(),Self::get(&data_type,data,length,version)?))
    }

    fn read_parmetic_curve(data:&[u8]) -> Result<Curve> {
        bound_check(data,0,12)?;
        let data_type = Self::read_data_type(data,0)?;
        if data_type != "para" {
            let mut ptr = 8;
            bound_check(data, ptr, 4)?;
            let mut curv = vec![];
            let count = read_u32_be(data, ptr) as usize;
            ptr += 4;
            bound_check(data, ptr, count * 2)?;
            for _ in 0..count {
                curv.push(read_u16_be(data, ptr));
                ptr += 2;
            }
            return Ok(Curve::Curve(curv))
        }

        let mut ptr = 8;
        let funtion_type = read_u16_be(data,ptr);
        ptr += 4;
        let mut vals :Vec<S15Fixed16Number> = vec![];
        
        let num = match funtion_type {
            0 => {
                1
            },
            1 => {
                3
            },
            2 => {
                4
            },
            3 => {
                5
            },
            4 => {
                7
            },
            _ => {  // Error
                0
            },
        };
        bound_check(data,ptr,num * 4)?;
        for _ in 0..num {
            vals.push(S15Fixed16Number{
                integer: read_i16_be(data, ptr),
                decimal: read_u16_be(data, ptr+2)
            });
            ptr += 4;
        }
        Ok(Curve::ParametricCurve(ParametricCurve{funtion_type,vals}))
    }

    fn read_formula_curve(data:&[u8]) -> Result<FormulaCurve> {
        bound_check(data,0,4)?;
        let mut ptr = 8;
        let funtion_type = read_u16_be(data,ptr);
        ptr += 4;
        let mut vals = vec![];
        
        let num = match funtion_type {
            0 => {
                4
            },
            1 => {
                5
            },
            2 => {
                5
            },
            _ => {  // Error
                0
            },
        };
        bound_check(data,ptr,num * 4)?;
        for _ in 0..num {
            vals.push(read_f32(data, ptr, Endian::BigEndian));
            ptr += 4;
        }
        Ok(FormulaCurve{funtion_type,vals})
    }

    fn read_localization(data:&[u8],ptr:usize,_version:u32) -> Result<LocalizedUnicode> {
        let mut ptr = ptr;
        bound_check(data, ptr, 12)?;
        let _ = read_u32_be(data,ptr);  // MUST 12
        ptr +=4;
        let lang = read_ascii_string(data,ptr,2);
        ptr +=2;
        let country = read_ascii_string(data,ptr,2);
        ptr +=2;
        let name_length = read_u32_be(data,ptr) as usize;
        ptr +=4;
        let name_offset = read_u32_be(data,ptr) as usize;
//        ptr +=4;
        let mut len = 0;
        bound_check(data, name_offset, name_length)?;
        let mut vals = vec![];
        while len < name_length {
            let val = read_u16_be(data, name_offset + len);
            if val == 0 {
                break;
            }
            vals.push(val);
            len += 2;
        }
        let unicode_string = String::from_utf16_lossy(&vals);
        let mult = LocalizedUnicode {
            lang,
            country,
            unicode_string
        };
        Ok(mult)
    }

    fn read_multi_localization(data:&[u8],ptr:usize,version:u32) -> Result<MultiLocalizedUnicodeType>{
        let mut ptr = ptr;
        bound_check(data, ptr, 20)?;
        let number_of_names = read_u32_be(data,ptr) as usize;
        ptr +=4;
        let mut unicode_strings = Vec::with_capacity(number_of_names);
        for _ in 0..number_of_names {
            let string =Self::read_localization(data, ptr, version)?;
            unicode_strings.push(string);
        }
        Ok(MultiLocalizedUnicodeType {
            unicode_strings
        })
    }

    pub fn get(data_type:&str,data: &[u8],length:usize,version:u32) -> Result<Data> {
        let len = length - 8;
        let mut ptr = 8;
        bound_check(data,ptr, len)?;
        match data_type {
            "para" => {
                let curve = Self::read_parmetic_curve(data)?;
                if let Curve::ParametricCurve(p_curve) = curve {
                    Ok(ParametricCurve(p_curve))
                } else {
                    Ok(None)
                }
            },
            "parf" => {
                Ok(FormulaCurve(Self::read_formula_curve(data)?))
            },
            "sig " => {
                let string = read_ascii_string(data, ptr, 4);
                Ok(ASCII(string))
            }
            "XYZ " | "XYZ" => {
                let mut xyzs :Vec<XYZNumber> = vec![];
                while  ptr < length {
                    let xyz = Self::xyz_number(data, ptr)?;
                    xyzs.push(xyz);
                    ptr += 12;
                }
                Ok(XYZNumberArray(xyzs))
            },
            "sf32" => { //s16Fixed16ArrayType
                let mut vals :Vec<S15Fixed16Number> = vec![];
                while  ptr < length {
                    vals.push(S15Fixed16Number{
                        integer: read_i16_be(data, ptr),
                        decimal: read_u16_be(data, ptr+2)
                    });
                    ptr += 4;
                }
                Ok(S15Fixed16NumberArray(vals))
            },
            "uf32" => { //U16Fixed16ArrayType
                let mut vals :Vec<U16Fixed16Number> = vec![];
                while  ptr < length {
                    vals.push(U16Fixed16Number{
                        integer: read_u16_be(data, ptr),
                        decimal: read_u16_be(data, ptr+2)
                    });
                    ptr += 4;
                }
                Ok(U16Fixed16NumberArray(vals))
            },
            "ui08" => { 
                let vals= read_bytes_as_vec(data, ptr,len);
                Ok(UInt8NumberArray(vals))
            },
            "ui16" => { 
                let mut vals= vec![];
                while  ptr < length {
                    vals.push(read_u16_be(data, ptr));
                    ptr += 2;
                }
                Ok(UInt16NumberArray(vals))
            },
            "ui32" => { 
                let mut vals= vec![];
                while  ptr < length {
                    vals.push(read_u32_be(data, ptr));
                    ptr += 4;
                }
                Ok(UInt32NumberArray(vals))
            },
            "ui64" => { 
                let mut vals= vec![];
                while  ptr < length {
                    vals.push(read_u64_be(data, ptr));
                    ptr += 8;
                }
                Ok(UInt64NumberArray(vals))
            },
            "text"=> {
                let string = read_ascii_string(data, ptr,len);
                Ok(Self::ASCII(string))
            },
            "desc" => {
                if version >= 0x40000000 {
                    let counts = read_u32_be(data, ptr) as usize;
                    ptr +=4;
                    bound_check(data,ptr,counts)?;
                    let ascii_string = read_ascii_string(data, ptr+4,counts);
                    ptr += counts;
                    bound_check(data,ptr,8)?;
                    let lang = read_ascii_string(data, ptr,4);
                    ptr += 4;
                    let counts = read_u32_be(data, ptr) as usize;
                    ptr +=4;
                    bound_check(data,ptr,counts)?;
                    // Unicode
                    let mut len = 0;
                    let mut vals = vec![];
                    while len < counts {
                        let val = read_u16_be(data, ptr);
                        if val == 0 {
                        break;
                        }
                        vals.push(val);
                        ptr += 2;
                        len += 2;
                    }
                    let local_string = String::from_utf16_lossy(&vals);
                    Ok(Descriptor(Descriptor{
                        ascii_string,
                        lang,
                        local_string,
                        // Macintosh Profile
                    }))
                } else {
                    let ascii_string = read_ascii_string(data, ptr+4,len-4);
                    Ok(ASCII(ascii_string))
                }
            }, 
            "chrm" => {
                let device_number = read_u16_be(data,ptr);
                let encoded_value = read_u16_be(data,ptr+2);
                ptr += 4;
                let mut vals :Vec<(U16Fixed16Number,U16Fixed16Number)> = vec![];
                while  ptr < length {
                    vals.push((
                        U16Fixed16Number{
                            integer: read_u16_be(data, ptr),
                            decimal: read_u16_be(data, ptr+2)
                        },
                        U16Fixed16Number{
                            integer: read_u16_be(data, ptr+2),
                            decimal: read_u16_be(data, ptr+4)
                        }));
                    ptr += 8;
                }
                Ok(Chromaticity(
                    ChromaticityType{
                        device_channels: device_number,
                        encoded_value: encoded_value,
                        cie_xy_coordinate_values: vals
                    }))
            },
            "mluc" |"vued" => {

                Ok(MultiLocalizedUnicode(
                    Self::read_multi_localization(data,ptr,version)?
                ))

            },
            "view" => {
                bound_check(data, ptr, 32)?;
                let illuminant = Self::xyz_number(data, ptr)?;
                ptr += 12;
                let surround = Self::xyz_number(data, ptr)?;
                ptr += 12;
                let illuminant_type = read_u32_be(data,ptr);
                Ok(ViewConditions(
                    ViewingConditions{
                        illuminant,
                        surround,
                        illuminant_type,
                    }
                    ))
            },
            "meas" => {
                bound_check(data, ptr, 28)?;
                let standard_observer = read_u32_be(data, ptr);
                ptr += 4;
                let measurement_backing = Self::xyz_number(data, ptr)?;
                ptr += 12;
                let measurement_geometry = read_u32_be(data, ptr);
                ptr += 4;
                let measurement_flare = U16Fixed16Number{
                    integer: read_u16_be(data, ptr),
                    decimal: read_u16_be(data, ptr+2)
                };
                ptr += 4;

                let standard_illuminant = read_u32_be(data, ptr);
                Ok(Measurement(
                    MeasurementType{
                        standard_observer,
                        measurement_backing,
                        measurement_geometry,
                        measurement_flare,
                        standard_illuminant
                }))
            },
            "curv" => {
                bound_check(data, ptr, 4)?;
                let mut curv = vec![];
                let count = read_u32_be(data, ptr) as usize;
                ptr += 4;
                bound_check(data, ptr, count * 2)?;
                for _ in 0..count {
                    curv.push(read_u16_be(data, ptr));
                    ptr += 2;
                }
                Ok(Curve(curv))
            },
            "mft1" | "mft2"  => {         
                bound_check(data, ptr,52)?;
                let input_channels= read_byte(data, ptr);
                ptr +=1;
                let output_channels = read_byte(data, ptr);
                ptr +=1;
                let number_of_clut_grid_points = read_byte(data, ptr);
                ptr +=2; // with skip padding

                let mut e_params:Vec<S15Fixed16Number> = vec![];
                // e00 e01 e02 ... e20 d21 e22
                for _ in 0..9 {
                    let e = S15Fixed16Number {
                        integer: read_i16_be(data, ptr),
                        decimal: read_u16_be(data, ptr+2)
                    };
                    e_params.push(e);
                    ptr += 4;
                }

                let clut_size = ((number_of_clut_grid_points as u32).pow(input_channels as u32) * output_channels as u32) as usize;

                if data_type == "mft1" {
                    let mut input_table = vec![];
                    let mut clut_values = vec![];
                    let mut output_table =vec![];

                    let input_channels_size = input_channels as usize * 256;
                    let output_channels_size = output_channels as usize * 256;

                    bound_check(data, ptr,input_channels_size + clut_size + output_channels_size)?;
                    for _ in 0..input_channels_size {
                        input_table.push(read_byte(data, ptr));
                        ptr += 1;
                    }

                    for _ in  0..clut_size {
                        clut_values.push(read_byte(data, ptr));
                        ptr += 1;
                    }

                    for _ in  0..output_channels_size {
                        output_table.push(read_byte(data, ptr));
                        ptr += 1;
                    }


                    let mft = Mft1{
                        input_channels,
                        output_channels,
                        number_of_clut_grid_points,
                        e_params,
                        input_table,
                        clut_values,
                        output_table, 

                    };
                    Ok(Lut8(mft))
                } else {
                    let mut input_table = vec![];
                    let mut clut_values = vec![];
                    let mut output_table =vec![];
                    bound_check(data, ptr,4)?;

                    let input_table_enteries = read_u16_be(data, ptr);
                    ptr += 2;
                    let output_table_enteries = read_u16_be(data, ptr);
                    ptr += 2;

                    let input_channels_size = input_channels as usize * input_table_enteries as usize;
                    let output_channels_size = output_channels as usize * output_table_enteries as usize;
                    bound_check(data, ptr,(input_channels_size + clut_size + output_channels_size) * 2)?;

                    for _ in 0..input_channels_size {
                        input_table.push(read_u16_be(data, ptr));
                        ptr += 2;
                    }

                    for _ in  0..clut_size {
                        clut_values.push(read_u16_be(data, ptr));
                        ptr += 2;
                    }

                    for _ in  0..output_channels_size {
                        output_table.push(read_u16_be(data, ptr));
                        ptr += 2;
                    }

                    let mft = Mft2{
                        input_channels,
                        output_channels,
                        number_of_clut_grid_points,
                        e_params,
                        input_table_enteries,
                        output_table_enteries,
                        input_table,
                        clut_values,
                        output_table, 

                    };
                    Ok(Lut16(mft))
                }
            },
            "mBA " | "mAB " => { // no sample 4.0
                bound_check(data, ptr,32)?;

                let input_channels= read_byte(data, ptr);
                ptr +=1;
                let output_channels = read_byte(data, ptr);
                ptr +=1;
                ptr +=2; // with skip padding
                let offset_b_curve = read_u32_be(data, ptr) as usize;
                ptr +=4;
                let offset_matrix = read_u32_be(data, ptr) as usize;
                ptr +=4;
                let offset_m_curve = read_u32_be(data, ptr) as usize;
                ptr +=4;
                let offset_clut = read_u32_be(data, ptr) as usize;
                ptr +=4;
                let offset_a_curve = read_u32_be(data, ptr) as usize;

                let mut b_curves = vec![];
                let mut m_curves = vec![];
                let mut a_curves = vec![];
                let mut matrix = vec![];

                let mut ptr = offset_b_curve;
                for _ in 0..input_channels {
                    let b_curve = Self::read_parmetic_curve(&data[ptr..])?;
                    ptr += b_curve.as_size();
                    b_curves.push(b_curve);
                }
                let mut ptr = offset_matrix;
                bound_check(data, ptr,12 *4)?;
                for _ in 0..12 {
                    let e = S15Fixed16Number {
                        integer: read_i16_be(data, ptr),
                        decimal: read_u16_be(data, ptr+2)
                    };
                    matrix.push(e);
                    ptr += 4;
                }
                let mut ptr = offset_m_curve;
                for _ in 0..input_channels {
                    let m_curve = Self::read_parmetic_curve(&data[ptr..])?;
                    ptr += m_curve.as_size();
                    m_curves.push(m_curve);
                }
                let mut ptr = offset_clut;
                bound_check(data, ptr,16)?;

                let mut grid_points = vec![];
                let mut clut_size = output_channels as usize;
                for i in 0..16 {
                    let grid_point = read_byte(&data,ptr+i);
                    if grid_point > 0 {
                        clut_size *= grid_point as usize;
                    } else {
                        break;
                    }
                    grid_points.push(grid_point);
                }
                ptr += 16;
                let precision = read_byte(&data,ptr);
                ptr += 4;   // with padding
                let clut_data = if precision == 1 {
                    let mut clut_entries:Vec<u8> = vec![];
                    bound_check(data, ptr,clut_size)?;
                    for _ in 0..clut_size {
                        clut_entries.push(read_byte(data, ptr));
                        ptr += 1;
                    }
                    Clut::UInt8(clut_entries)
                } else {
                    let mut clut_entries:Vec<u16> = vec![];
                    bound_check(data, ptr,clut_size * 2)?;
                    for _ in 0..clut_size {
                        clut_entries.push(read_u16_be(data, ptr));
                        ptr += 2;
                    }
                    Clut::UInt16(clut_entries)
                };

                let clut = MClut {
                    grid_points,
                    precision,
                    clut_data,
                };

                let mut ptr = offset_a_curve;
                for _ in 0..input_channels {
                    let a_curve = Self::read_parmetic_curve(&data[ptr..])?;
                    ptr += a_curve.as_size();
                    a_curves.push(a_curve);
                }
                if data_type == "mBA " {

                    Ok(LutBtoA(Mba{
                        input_channels,
                        output_channels,
                        b_curves,
                        matrix,
                        m_curves,
                        clut,
                        a_curves,
                    }))
                } else {
                    Ok(LutAtoB(Mab{
                        input_channels,
                        output_channels,
                        b_curves,
                        matrix,
                        m_curves,
                        clut,
                        a_curves,
                    }))

                }
            },
            "rcs2" => {
                bound_check(data, ptr, 4)?;
                let number_of_channels = read_u16_be(data, ptr);
                ptr += 2;
                let count_of_measirement_types = read_u16_be(data, ptr);
                let mut count_relative_offsets = vec![];
                for _ in 0..count_of_measirement_types {
                    count_relative_offsets.push(read_u32_be(data, ptr));
                    ptr += 4;
                }
                let mut response_curve_structures = vec![];
                for offset in count_relative_offsets {
                    let mut offset = offset as usize;
                    bound_check(data, offset, number_of_channels as usize * (4 + 12 + 8) + 4)?;
                    let signature = read_u32_be(data,offset);
                    offset += 4;
                    let mut for_each_channel = vec![];
                    for _ in 0..number_of_channels {
                        let val = read_u32_be(data,offset);
                        offset += 4;
                        for_each_channel.push(val);
                    }
                    let mut patch_with_the_maximum_colorant_value = vec![];
                    for _ in 0..number_of_channels {
                        let val = Self::xyz_number(data, offset)?;
                        offset += 12;
                        patch_with_the_maximum_colorant_value.push(val);                        
                    }
                    let mut response_arrays = vec![];
                    for _ in 0..number_of_channels {
                        let encoding_the_interval = read_u16_be(data, offset);
                        offset += 2;
                        let reserved = read_u16_be(data, offset);
                        offset += 2;
                        let measurement_value = S15Fixed16Number {
                            integer: read_i16_be(data, offset),
                            decimal: read_u16_be(data, offset+2)
                        };
                        offset += 4;
                        response_arrays.push(Response16Number{
                            encoding_the_interval,
                            reserved,
                            measurement_value,
                        });
                    }
                    response_curve_structures.push(CurveStructure{
                        signature,
                        for_each_channel,
                        patch_with_the_maximum_colorant_value,
                        response_arrays,
                    });
                }
                Ok(Self::ResponseCurveSet16(ResponseCurveSet16{
                    number_of_channels,
                    count_of_measirement_types,
                    response_curve_structures,
                }))
            },
            "crdi" => {
                let mut strings = vec![];
                while ptr < length {
                    bound_check(data, ptr, 4)?;
                    let strlen = read_u32_be(data, ptr) as usize;
                    ptr += 4;
                    bound_check(data, ptr, strlen)?;
                    strings.push(read_ascii_string(data, ptr, strlen));
                    ptr += strlen;
                }
                Ok(CrdInfo(strings))
            },
            "data" => {
                bound_check(data, ptr, 4)?;
                let data_type = read_u32_be(data, ptr) as usize;
                ptr += 4;
                bound_check(data, ptr, len-4)?;
                if data_type  == 0 {
                    let string = read_ascii_string(data, ptr, len -4);
                    Ok(ASCII(string))
                } else {
                    let raw = read_bytes_as_vec(data, ptr, len -4);
                    Ok(Binary(raw))
                }

            },
            "clro" => {
                bound_check(data, ptr, 4)?;
                let counts = read_u32_be(data, ptr) as usize;
                ptr += 4;
                let count  = read_u32_be(data, ptr) as usize;
                bound_check(data, ptr, count)?;
                let raw = read_bytes_as_vec(data, ptr, counts);
                Ok(UInt8NumberArray(raw))
            },
            "clrt" => {
                bound_check(data, ptr, 38)?;
                let counts = read_u32_be(data, ptr) as usize;
                ptr += 4;
                let colorant_name = read_ascii_string(data, ptr, 32);
                ptr += 32;
                bound_check(data, ptr, counts*6)?;
                let mut pcs_values: Vec<[u16;3]> = Vec::with_capacity(counts);
                for _ in 0..counts {
                    let mut pcs = [0_u16;3];
                    pcs[0] = read_u16_be(data, ptr);
                    pcs[1] = read_u16_be(data, ptr+2);
                    pcs[2] = read_u16_be(data, ptr+4);
                    ptr += 6;
                    pcs_values.push(pcs);
                }

                Ok(ColorantTable(ColorantTableType{colorant_name,pcs_values}))

            },
            "mpet" => {
                bound_check(data, ptr, 8)?;
                let input_channels = read_u16_be(data, ptr);
                let output_channels = read_u16_be(data, ptr+2);
                let counts = read_u32_be(data, ptr+4) as usize;
                ptr += 8;
                bound_check(data, ptr, counts*8)?;
                let mut process_element_positions = Vec::with_capacity(counts);
                for _ in 0..counts {
                    let offset = read_u32_be(data, ptr);
                    let size = read_u32_be(data, ptr+4);
                    ptr += 8;
                    process_element_positions.push(PositionNumber{offset,size});
                }
                let data = read_bytes_as_vec(data,ptr,length - ptr);

                Ok(MultiProcessElements(
                    MultiProcessElementsType {
                        input_channels,
                        output_channels,
                        process_element_positions,
                        data
                    }
                ))
            },
            "cvst" => {
                bound_check(data, ptr, 4)?;
                let input_channels = read_u16_be(data, ptr);
                let output_channels = read_u16_be(data, ptr+2);
                let counts = input_channels as usize;
                ptr += 4;
                bound_check(data, ptr, counts*8)?;
                let mut curve_positions = Vec::with_capacity(counts);
                for _ in 0..counts {
                    let offset = read_u32_be(data, ptr);
                    let size = read_u32_be(data, ptr+4);
                    ptr += 8;
                    curve_positions.push(PositionNumber{offset,size});
                }
                let data = read_bytes_as_vec(data,ptr,length - ptr);

                Ok(CurveSet(
                    CurveSetType {
                        input_channels,
                        output_channels,
                        curve_positions,
                        data
                    }
                ))
            },
            "curf" => { // 4.x
                bound_check(data, ptr, 4)?;
                let segments = read_u16_be(data, ptr);
                ptr += 4;   // skip
                let counts = segments as usize - 1;
                let mut dimensional_curves = Vec::with_capacity(segments as usize);
                for _ in 0..segments as usize {
                    bound_check(data, ptr, counts*4)?;
                    let mut  break_points = Vec::with_capacity(counts);
                    for _ in 0..counts {
                        let float = read_f32(data, ptr, Endian::BigEndian);
                        ptr += 4;
                        break_points.push(float);
                    }
                    if ptr + counts * 4 > length {break}
                    dimensional_curves.push(break_points);
                }

                Ok(OneDimenstionalCurves(
                    OneDimensionalCurvesType {
                        segments,
                        dimensional_curves,
                    }
                ))
            },
            "samf" => { // 4.x
                bound_check(data, ptr, 4)?;
                let counts = read_u16_be(data, ptr) as usize;
                ptr += 4;   // skip
                bound_check(data, ptr, counts * 4)?;
                let mut curve_entries = vec![];
                for _ in 0..counts {
                    let float = read_f32(data, ptr, Endian::BigEndian);
                    ptr += 4;
                    curve_entries.push(float);
                }
                Ok(SampledCurve(curve_entries))
            },
            "matf" => { // 4.x
                bound_check(data, ptr, 4)?;
                let input_channels = read_u16_be(data, ptr);
                let output_channels = read_u16_be(data, ptr+2);
                ptr += 4;
                let counts = (input_channels as usize + 1) * output_channels as usize;
                bound_check(data, ptr, counts*4)?;
                let mut matrix = vec![];
                for _ in 0..counts {
                    let float = read_f32(data, ptr, Endian::BigEndian);
                    ptr += 4;
                    matrix.push(float);
                }
                Ok(MatrixElement(MatrixElement{input_channels,output_channels,matrix}))
            },
            "ncl2" => { // ncol obsolute
                bound_check(data, ptr, 122)?;
                let vendor_specific_flag = read_u32_be(data, ptr);
                ptr += 4;
                let counts = read_u32_be(data, ptr) as usize;
                ptr += 4;
                let number_device_coordinates = read_u32_be(data, ptr) as usize;
                ptr += 4;
                let prefix_color_name = read_ascii_string(data, ptr,32);
                ptr += 32;
                let suffix_color_name = read_ascii_string(data, ptr,32);
                ptr += 32;
                let mut entries = vec![];
                for _ in 0..counts {
                    bound_check(data, ptr, number_device_coordinates * 2 + 32 + 6)?;
                    let root_color_name = read_ascii_string(data, ptr,32);
                    ptr += 32;
                    let mut pcs = [0_u16;3];
                    pcs[0] = read_u16_be(data, ptr);
                    pcs[1] = read_u16_be(data, ptr+2);
                    pcs[2] = read_u16_be(data, ptr+4);
                    ptr += 6;
                    let mut coordinate = vec![];
                    for _ in 0..number_device_coordinates {
                        coordinate.push(read_u16_be(data, ptr));
                        ptr += 2;
                    }
                    entries.push(ColorNameWithData{
                        root_color_name,
                        pcs,
                        coordinate,
                    });
                }
                Ok(NamedColor(NamedColor2Type{
                    vendor_specific_flag,
                    prefix_color_name,
                    suffix_color_name,
                    entries,
                }))
            }
            "pseq" => {
                bound_check(data,ptr,4)?;
                let counts = read_u32_be(data, ptr) as usize;
                ptr += 4;
                bound_check(data,ptr,counts)?;
                let mut remain = length - ptr;
                let mut profiles = vec![];
                for _ in 0..counts {
                    bound_check(data,ptr,20)?;
                    if remain <= 0 {break;}
                    let device_manufacturer = read_u32_be(data, ptr);
                    ptr += 4;
                    let device_model = read_u32_be(data, ptr);
                    ptr += 4;
                    let device_attributes = read_ascii_string(data, ptr,8);
                    ptr += 8;
                    let technology_information = read_u32_be(data, ptr);
                    ptr += 4;
                    remain -=20;
                    let _tag = read_u32_be(data, ptr); // dmnd
                    ptr += 4;
                    let len = read_u32_be(data, ptr) as usize;
                    ptr += 4;
                    bound_check(data, ptr, len)?;
                    let (_,res) = Self::parse(&data[ptr..],len, version)?;
                    let description_device_manufacturer;
                    match res {
                        ASCII(text) => {
                            description_device_manufacturer = MultiLocalizedUnicodeType::from(text);

                        },
                        MultiLocalizedUnicode(mlut) => {
                            description_device_manufacturer = mlut;
                        }
                        _ => {
                            description_device_manufacturer = MultiLocalizedUnicodeType::from("".to_string());
                        }
                    }
                    ptr += len;
                    let _tag = read_u32_be(data, ptr); // dmnd
                    ptr += 4;
                    let len = read_u32_be(data, ptr) as usize;
                    ptr += 4;
                    bound_check(data, ptr, len)?;
                    let (_,res) = Self::parse(&data[ptr..],len, version)?;
                    let description_device_model;
                    match res {
                        ASCII(text) => {
                            description_device_model = MultiLocalizedUnicodeType::from(text);

                        },
                        MultiLocalizedUnicode(mlut) => {
                            description_device_model = mlut;
                        }
                        _ => {
                            description_device_model = MultiLocalizedUnicodeType::from("".to_string());
                        }
                    }
                    profiles.push(ProfileDescription{
                        device_manufacturer,
                        device_model,
                        device_attributes,
                        technology_information,
                        description_device_manufacturer,
                        description_device_model,
                    })
                }
                Ok(ProfileDescription(profiles))
            }

            _ => { // Data type
                // no impl

                // "ncol" // 3.2 obsolute?
                // "scrn" // 3.2 obsolute 4.0?
                // "clut" // 4.0
                // "bACS" // 4.0
                // "eACS" // 4.0

                // "dict" // 5.0
                // "ehim" // 5.0
                // "enim" // 5.0
                // "fl16" // 5.0
                // "fl32" // 5.0
                // "fl64" // 5.0
                // "gbd " // 5.0
                // "smat" // 5.0
                // "svcn" // 5.0
                // "tary" // 5.0
                // "tstr" // 5.0
                // "utf8" // 5.0
                // "zut8" // 5.0
                // "zxml" // 5.0

                let raw = read_bytes_as_vec(data, ptr, len);
                Ok(Raw(data_type.to_string(),raw))
            }
        }

    }

    pub fn as_string(&self,verbose:usize) -> String{
        match &*self {
            DataTimeNumber(datetime) => {
                datetime.as_string()
            },
            Float32Number(f) => {
                f.to_string()
            },
            PositionNumber(position) => {
                format!("offset {} size {}",position.offset,position.size)
            },
            S15Fixed16Number(f) => {
                f.as_f32().to_string()
            },
            S15Fixed16NumberArray(vals) => {
                let mut str = "".to_string();
                for f in vals {
                    str += &f.as_f32().to_string();
                    str += " ";
                }
                str.to_string()
            },
            ParametricCurve(parametic_curve) => {
                let mut str = match parametic_curve.funtion_type {
                    0x000 => {"function Y = X**ganma\n"},
                    0x001 => {"function Y = (aX+b)**ganma (X >= -b/a), Y = 0 (X < -b/a)\n"},
                    0x002 => {"function Y = (aX+b)**ganma + c(X >= -b/a), Y = c (X < -b/a)\n"},
                    0x003 => {"function Y = (aX+b)**ganma (X >= d), Y = cX (X < d)\n"},
                    0x004 => {"function Y = (aX+b)**ganma + e(X >= d), Y = cX + f (X < d)\n"},
                    _ => {"function Unknown"},
                }.to_string();
                for f in &parametic_curve.vals {
                    str += &f.as_f32().to_string();
                    str += " ";
                }
                str.to_string()
            }
            U16Fixed16Number(f) => {
                f.as_f32().to_string()
            },
            Response16Number(v) => {
                format!("{} {} {}",v.encoding_the_interval,v.reserved,v.measurement_value.as_f32())              
            },
            U1Fixed15Number(f) => {
                f.as_f32().to_string()
            },
            U8Fixed8Number(f) => {
                f.as_f32().to_string()
            },
            UInt8Number(n) => {
                n.to_string()
            },
            UInt16Number(n) => {
                n.to_string()
            },
            UInt32Number(n) => {
                n.to_string()
            },
            UInt64Number(n) => {
                n.to_string()
            },
            XYZNumber(xyz) => {
                format!("X:{} Y:{} Z:{}\n",xyz.x.as_f32(),xyz.y.as_f32(),xyz.z.as_f32())
            },
            XYZNumberArray(xyzs) => {
                let mut str = "".to_string();
                for xyz in xyzs {
                    str += &format!("X:{} Y:{} Z:{} ",xyz.x.as_f32(),xyz.y.as_f32(),xyz.z.as_f32())
                }
                str + "\n"
            },
            Chromaticity(chromaticity) => {
                let encoded = match chromaticity.encoded_value {
                    0x001 => {"ITU-R BT.709"},
                    0x002 => {"SMPTE RP145-1994"},
                    0x003 => {"EBU Tech.3213-E"},
                    0x004 => {"P22"},
                    _ => {"unknown"},

                };
                let mut str = format!("Number of Device Channels {} {} ",chromaticity.device_channels,encoded);
                for (x,y) in &chromaticity.cie_xy_coordinate_values {
                    str += &format!("x:{} y:{} ",x.as_f32(),y.as_f32());
                }

                str + "\n"
            },
            Measurement(mesaurement) => {
                let mut str = match mesaurement.standard_observer {
                    0x00000001 => {"Standard Observer: CIE 1931 standard colorimetric observer\n"},
                    0x00000002 => {"Standard Observer: CIE 1964 standard colorimetric observer\n"},
                    _ => {"Standard: Observer unknown\n"},
                }.to_string();
                str += &format!("XYZ tristimulus values {}\n",mesaurement.measurement_backing.to_string());
                str += "Measurement geometry ";
                str += match mesaurement.measurement_geometry {
                    0x00000001 => {"0/45 or 45/0\n"},
                    0x00000002 => {"0/d or d/0\n"},
                    _ => {"unknown\n"},
                };
                str += &format!("Measurement flare {}\n",mesaurement.measurement_flare.as_f32());
                str += "Standard Illuminant: ";
                str += &illuminant_type_string(mesaurement.standard_illuminant);
                str + "\n"
            },
            ResponseCurveSet16(response_curve_structures) => {
                let mut str = "".to_string();
                str += &format!("number_of_channels {} count_of_measirement_types{}\n",
                    response_curve_structures.number_of_channels,
                    response_curve_structures.count_of_measirement_types);
                
                    for curve_structure in &response_curve_structures.response_curve_structures {
                        let sig = read_ascii_string(&curve_structure.signature.to_be_bytes(),0,4);

                        str += &format!("{}\n",sig);
                        
                        for each in &curve_structure.for_each_channel {
                            str += &format!("{} ",each)
                        }
                        str += "\n";

                        for xyz in &curve_structure.patch_with_the_maximum_colorant_value {
                            str += &format!("X:{} Y:{} Z:{} ",xyz.x.as_f32(),xyz.y.as_f32(),xyz.z.as_f32())
                        }
                        str += "\n";

                        for respose in &curve_structure.response_arrays {
                            str += &format!("{:?} ",respose)
                        }
                        str += "\n";
                    }

                str + "\n"
            }

            ASCII(string) => {
                string.to_string()
            },
            ViewConditions(condition) => {
                format!("illuminant {} surround {} type {}",
                    condition.illuminant.to_string(),
                    condition.surround.to_string(),
                    illuminant_type_string(condition.illuminant_type))
            },
            Lut8(lut) => {
                let mut str = format!("Lut8 input #{} output #{} grid {}\n",
                    lut.input_channels,
                    lut.output_channels,
                    lut.number_of_clut_grid_points
                );
                let e = &lut.e_params;

                if e.len() > 9 {
                    str += &format!("|{} {} {}|\n",e[0].as_f32(),e[1].as_f32(),e[2].as_f32());
                    str += &format!("|{} {} {}|\n",e[3].as_f32(),e[4].as_f32(),e[5].as_f32());
                    str += &format!("|{} {} {}|\n",e[6].as_f32(),e[7].as_f32(),e[8].as_f32());
                }
                str += &format!("input table size {} clut values {}  output table size {}\n",
                    lut.input_table.len(),lut.clut_values.len(),lut.output_table.len());
                if verbose > 0 {
                    let len = lut.input_table.len() / lut.input_channels as usize;
                    str += &format!("Input table\n");
                    for j in 0..lut.input_channels as usize {
                        str += &format!("{:3}: ", j);
                        for i in 0..len {
                            str += &format!("{:3} ", lut.input_table[j*len + i]);
                        }
                        str += &format!("\n");
                    }
                    str += &format!("Clut values\n");
                    let mut nums = vec![0_usize;lut.input_channels as usize];
                    let mut i = 0;
                    while i < lut.clut_values.len() as usize {
                        let mut current = i / lut.output_channels as usize;
                        for j in 0..lut.input_channels as usize {
                            nums[j] = current % lut.number_of_clut_grid_points as usize;
                            current /= lut.number_of_clut_grid_points as usize;
                        }
                        for j in 0..lut.input_channels as usize {
                            str += &format!("{:3} ",nums[lut.input_channels as usize - j -1 ]);
                        }
                        str += ": ";

                        for j in 0..lut.output_channels as usize {
                            str += &format!("{:3} ",lut.clut_values[i+j]);
                        }

                        i += lut.output_channels as usize;
                        str += &format!("\n");
                    }

                    let len = lut.output_table.len() / lut.output_channels as usize;
                    str += &format!("\nOutput table\n");
                    for j in 0..lut.output_channels as usize {
                        str += &format!("{:3}: ", j);
                        for i in 0..len {
                            str += &format!("{:3} ", lut.output_table[j*len + i]);
                        }
                        str += &format!("\n");
                    }
                }

                str.to_string()
            }
            Lut16(lut) => {
                let mut str = format!("Lut16 input #{} output #{} grid {}\n",
                    lut.input_channels,
                    lut.output_channels,
                    lut.number_of_clut_grid_points
                );
                let e = &lut.e_params;

                if e.len() >= 9 {
                    str += &format!("|{} {} {}|\n",e[0].as_f32(),e[1].as_f32(),e[2].as_f32());
                    str += &format!("|{} {} {}|\n",e[3].as_f32(),e[4].as_f32(),e[5].as_f32());
                    str += &format!("|{} {} {}|\n",e[6].as_f32(),e[7].as_f32(),e[8].as_f32());
                }

                str += &format!("input table entries {} output table entries {}\n",
                    lut.input_table_enteries,
                    lut.output_table_enteries);

                str += &format!("input table size {} clut values {}  output table size {}\n",
                    lut.input_table.len(),lut.clut_values.len(),lut.output_table.len());
                if verbose > 0 {
                    let len = lut.input_table.len() / lut.input_channels as usize;
                    str += &format!("Input table\n");
                    for j in 0..lut.input_channels as usize {
                        str += &format!("{:3}: ", j);
                        for i in 0..len {
                            str += &format!("{:5} ", lut.input_table[j*len + i]);
                        }
                        str += &format!("\n");
                    }
                    str += &format!("Clut values\n");
                    let mut nums = vec![0_usize;lut.input_channels as usize];
                    let mut i = 0;
                    while i < lut.clut_values.len() as usize {
                        let mut current = i / lut.output_channels as usize;
                        for j in 0..lut.input_channels as usize {
                            nums[j] = current % lut.number_of_clut_grid_points as usize;
                            current /= lut.number_of_clut_grid_points as usize;
                        }
                        for j in 0..lut.input_channels as usize {
                            str += &format!("{:3} ",nums[lut.input_channels as usize - j -1 ]);
                        }
                        str += ": ";

                        for j in 0..lut.output_channels as usize {
                            str += &format!("{:5} ",lut.clut_values[i+j]);
                        }

                        i += lut.output_channels as usize;
                        str += &format!("\n");
                    }


                    let len = lut.output_table.len() / lut.output_channels as usize;
                    str += &format!("\nOutput table\n");
                    for j in 0..lut.output_channels as usize {
                        str += &format!("{:3}: ", j);
                        for i in 0..len {
                            str += &format!("{:5} ", lut.output_table[j*len + i]);
                        }
                        str += &format!("\n");
                    }
                }
                str
            },
            LutBtoA(lut) => {
                let mut str = format!("Lut B to A input #{} output #{}\n",
                    lut.input_channels, lut.output_channels);

                str += "B Curves\n";


                for curve in &lut.b_curves {
                    str += &curve.to_string();
                }

                let e = &lut.matrix;
                str += "Matrix\n";

                if e.len() >= 12 {
                    str += &format!("|{} {} {}| |{}|\n",e[0].as_f32(),e[1].as_f32(),e[2].as_f32(),e[9].as_f32() );
                    str += &format!("|{} {} {}| |{}|\n",e[3].as_f32(),e[4].as_f32(),e[5].as_f32(),e[10].as_f32());
                    str += &format!("|{} {} {}| |{}|\n",e[6].as_f32(),e[7].as_f32(),e[8].as_f32(),e[11].as_f32());
                }

                str += "M Curves\n";
                for curve in &lut.m_curves {
                    str += &curve.to_string();
                }

                str += "CLUT\n";
                let clut = &lut.clut;
                if verbose > 0 {
                    str += &format!("{:?} Precision {} {:?}\n",clut.grid_points,clut.precision,clut.clut_data);
                } else {
                    str += &format!("{:?} Precision {} CLUT {} entries\n",clut.grid_points,clut.precision,clut.len());
                }

                str += "A Curves\n";
                for curve in &lut.m_curves {
                    str += &curve.to_string();
                }

                str.to_string()
            },
            LutAtoB(lut) => {
                let mut str = format!("Lut A to B input #{} output #{}\n",
                    lut.input_channels, lut.output_channels);

                str += "B Curves\n";


                for curve in &lut.b_curves {
                    str += &curve.to_string();
                }

                let e = &lut.matrix;
                str += "Matrix\n";

                if e.len() >= 12 {
                    str += &format!("|{} {} {}| |{}|\n",e[0].as_f32(),e[1].as_f32(),e[2].as_f32(),e[9].as_f32() );
                    str += &format!("|{} {} {}| |{}|\n",e[3].as_f32(),e[4].as_f32(),e[5].as_f32(),e[10].as_f32());
                    str += &format!("|{} {} {}| |{}|\n",e[6].as_f32(),e[7].as_f32(),e[8].as_f32(),e[11].as_f32());
                }

                str += "M Curves\n";
                for curve in &lut.m_curves {
                    str += &curve.to_string();
                }

                let clut = &lut.clut;

                if verbose > 0 {
                    str += &format!("{:?} Precision {} {:?}\n",clut.grid_points,clut.precision,clut.clut_data);
                } else {
                    str += &format!("{:?} Precision {} CLUT {} entries\n",clut.grid_points,clut.precision,clut.len());
                }

                str += "A Curves\n";
                for curve in &lut.m_curves {
                    str += &curve.to_string();
                }

                str.to_string()
            },
            Curve(curve) => {
                let str = format!("Curve table size {}\n",curve.len());
                str
            },
            MultiLocalizedUnicode(mult) => {
                mult.list_string()
            },
            Raw(data_type,data) => {
                if verbose == 0 {
                    format!("Unimplement data type {} length {}bytes\n",data_type,data.len())
                } else {
                    format!("Unimplement data type {} {:?}\n",data_type,data)
                }
            }
            None => {
                "None".to_string()
            },
            _ => {
                format!("{:?}",*self)
            }
        }
    }


    pub fn xyz_number(data: &[u8],ptr: usize) ->  Result<XYZNumber> {
        bound_check(data, ptr, 12)?;
        let cie_x = S15Fixed16Number {
            integer: read_i16_be(&data, ptr),
            decimal: read_u16_be(&data, ptr+2)
        };
        let cie_y = S15Fixed16Number {
            integer: read_i16_be(&data, ptr+4),
            decimal: read_u16_be(&data, ptr+6)
        };
        let cie_z = S15Fixed16Number {
            integer: read_i16_be(&data, ptr+8),
            decimal: read_u16_be(&data, ptr+10)
        };
        Ok(XYZNumber{x:cie_x,y:cie_y,z:cie_z})
    }

    pub fn read_data_type(data:&[u8],ptr: usize) -> Result<String> {
        bound_check(data, ptr, 4)?;
        let data_type = read_string(data, ptr, 4);
        if data_type.len() == 0 {
            return Ok(read_string(data, ptr + 1, 3))
        }
        Ok(data_type)
    }
}

