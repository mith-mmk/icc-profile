//! Legacy tag inspection is not a complete ICC conformance validator.
use icc_profile::iccprofile::{Data, Curve, ICCNumber};
const VERSION: u32 = 0x04000000;
const KINDS: &[&str] = &["para", "parf", "sig ", "XYZ ", "sf32", "uf32", "ui08", "ui16", "ui32", "ui64", "text", "desc", "chrm", "mluc", "vued", "view", "meas", "curv", "mft1", "mft2", "mAB ", "mBA ", "rcs2", "crdi", "data", "clro", "clrt", "mpet", "cvst", "curf", "samf", "matf", "ncl2", "pseq", "zzzz"];
fn tag(kind: &str, n: usize) -> Vec<u8> { let mut v=vec![0;n]; v[..4].copy_from_slice(kind.as_bytes()); v }
fn u16be(v: &mut [u8], p: usize, n: u16) { v[p..p+2].copy_from_slice(&n.to_be_bytes()); }
fn u32be(v: &mut [u8], p: usize, n: u32) { v[p..p+4].copy_from_slice(&n.to_be_bytes()); }
fn parse(v: &[u8]) -> Data { Data::parse(v, v.len(), VERSION).unwrap().1 }
fn formula() -> Vec<u8> { let mut v=tag("parf",28); for (i,n) in [1.0f32,2.0,3.0,4.0].iter().enumerate() { u32be(&mut v,12+i*4,n.to_bits()); } v }
fn curve() -> Vec<u8> { let mut v=tag("curv",16); u32be(&mut v,8,1); u16be(&mut v,12,256); v }
fn mluc() -> Vec<u8> { let mut v=tag("mluc",44); u32be(&mut v,8,2); u32be(&mut v,12,12); v[16..20].copy_from_slice(b"enUS"); u32be(&mut v,20,2); u32be(&mut v,24,40); v[28..32].copy_from_slice(b"jaJP"); u32be(&mut v,32,2); u32be(&mut v,36,42); u16be(&mut v,40,65); u16be(&mut v,42,0x3042); v }
fn segmented() -> Vec<u8> { let mut v=tag("curf",12); u16be(&mut v,8,1); v.extend(formula()); v }
fn positions(kind: &str, payload: Vec<u8>) -> Vec<u8> { let base=if kind=="mpet" {24}else{20}; let mut v=tag(kind,base); u16be(&mut v,8,1); u16be(&mut v,10,1); let p=if kind=="mpet" { u32be(&mut v,12,1);16 }else{12}; u32be(&mut v,p,base as u32); u32be(&mut v,p+4,payload.len() as u32); v.extend(payload); v }
#[test]
fn deterministic_truncations_and_count_mutations_never_panic() {
    for &kind in KINDS {
        let mut bytes=tag(kind,192);
        for size in 0..=192 {
            let sliced=&bytes[..size];
            let direct=Data::get(kind,&bytes,size,VERSION);
            let clipped=Data::get(kind,sliced,size,VERSION);
            assert_eq!(format!("{direct:?}"),format!("{clipped:?}"),"declared length: {kind}/{size}");
            let _=Data::parse(sliced,size,VERSION);
        }
        for offset in (8..188).step_by(2) {
            for value in [1, 2, 0xffff, 0x7fffffff, u32::MAX] {
                u32be(&mut bytes,offset,value);
                let _=Data::get(kind,&bytes,bytes.len(),VERSION);
                u32be(&mut bytes,offset,0);
            }
        }
        assert!(Data::get(kind,&bytes,usize::MAX,VERSION).is_err());
        assert!(Data::parse(&bytes,usize::MAX,VERSION).is_err());
    }
    for offset in [0, 1, 3, usize::MAX] {
        assert!(Data::xyz_number(&[0;3],offset).is_err());
        assert!(Data::read_data_type(&[0;3],offset).is_err());
    }
}
#[test]
fn exact_localization_formula_and_chromaticity_values() {
    match parse(&mluc()) { Data::MultiLocalizedUnicode(v)=> { assert_eq!(v.unicode_strings.len(),2); assert_eq!(v.unicode_strings[0].unicode_string,"A"); assert_eq!(v.unicode_strings[1].unicode_string,"あ"); assert_eq!(v.unicode_strings[1].lang,"ja"); }, x=>panic!("{x:?}") }
    let mut bad=mluc(); u32be(&mut bad,32,1); assert!(Data::parse(&bad,bad.len(),VERSION).is_err());
    u32be(&mut bad,32,2); u16be(&mut bad,42,0xd800); assert!(Data::parse(&bad,bad.len(),VERSION).is_err());
    match parse(&formula()) { Data::FormulaCurve(v)=>assert_eq!(v.vals,[1.,2.,3.,4.]),x=>panic!("{x:?}") }
    let mut c=tag("chrm",20); u16be(&mut c,8,1); for (i,n) in [1,2,3,4].iter().enumerate(){u16be(&mut c,12+i*2,*n);}
    match parse(&c) { Data::Chromaticity(v)=> {let (x,y)=&v.cie_xy_coordinate_values[0]; assert_eq!((x.int(),x.decimal(),y.int(),y.decimal()),(1,2,3,4));},x=>panic!("{x:?}") }
}
#[test]
fn array_partial_elements_and_formula_shortage_are_errors() {
    for (kind,stride) in [("XYZ ",12),("sf32",4),("uf32",4),("ui16",2),("ui32",4),("ui64",8)] { for extra in 1..stride { let v=tag(kind,8+stride+extra); assert!(Data::parse(&v,v.len(),VERSION).is_err()); } }
    let v=formula(); for n in 0..v.len() { assert!(Data::get("parf",&v,n,VERSION).is_err()); }
    for kind in ["para","parf"] { let mut v=tag(kind,64); u16be(&mut v,8,65535); assert!(Data::parse(&v,v.len(),VERSION).is_err()); }
}
#[test]
fn nested_lut_channels_padding_offsets_and_optional_stages() {
    for kind in ["mAB ","mBA "] {
        let mut v=tag(kind,32); v[8]=if kind=="mAB " {2}else{1}; v[9]=if kind=="mAB " {1}else{2}; u32be(&mut v,12,32); v.extend(curve());
        match parse(&v) {Data::LutAtoB(v)=> {assert_eq!(v.b_curves.len(),1); assert_eq!(v.clut.len(),0);},Data::LutBtoA(v)=>assert_eq!(v.b_curves.len(),1),x=>panic!("{x:?}")}
        for offset in [1,31,33,1000,u32::MAX] { let mut bad=v.clone();u32be(&mut bad,12,offset);assert!(Data::parse(&bad,bad.len(),VERSION).is_err()); }
    }
    let mut v=tag("mAB ",32);v[8]=2;v[9]=2;u32be(&mut v,12,32);v.extend(curve());v.extend(curve());
    match parse(&v){Data::LutAtoB(v)=> { assert_eq!(v.b_curves.len(),2);assert!(matches!(&v.b_curves[1],Curve::Curve(n) if n==&[256]));},x=>panic!("{x:?}")}
    v[48..52].copy_from_slice(b"oops");assert!(Data::parse(&v,v.len(),VERSION).is_err());
}
#[test]
fn nested_mpe_and_segment_payloads_are_validated() {
    let v=positions("mpet",positions("cvst",segmented())); assert!(matches!(parse(&v),Data::MultiProcessElements(_)));
    for n in 0..v.len(){assert!(Data::get("mpet",&v,n,VERSION).is_err());}
    let mut bad=v.clone();let last=bad.len()-1;bad.truncate(last);assert!(Data::parse(&bad,bad.len(),VERSION).is_err());
    let mut bad=segmented();u16be(&mut bad,8,0);assert!(Data::parse(&bad,bad.len(),VERSION).is_err());
    let unknown=positions("mpet",tag("zzzz",12));assert!(Data::parse(&unknown,unknown.len(),VERSION).is_err());
    assert!(matches!(parse(&tag("zzzz",12)),Data::Raw(_, _)));
}
#[test]
fn colorant_records_and_response_counts_follow_wire_layout() {
    let mut v=tag("clrt",88);u32be(&mut v,8,2);v[12..16].copy_from_slice(b"Cyan");v[50..54].copy_from_slice(b"Pink");for (p,n) in [(44,1),(46,2),(48,3),(82,4),(84,5),(86,6)]{u16be(&mut v,p,n);}
    match parse(&v){Data::ColorantTable(v)=>assert_eq!(v.pcs_values,[[1,2,3],[4,5,6]]),x=>panic!("{x:?}")}
    let mut v=tag("clro",15);u32be(&mut v,8,3);v[12..15].copy_from_slice(&[2,0,1]);match parse(&v){Data::UInt8NumberArray(v)=>assert_eq!(v,[2,0,1]),x=>panic!("{x:?}")}
    let mut v=tag("rcs2",52);u16be(&mut v,8,1);u16be(&mut v,10,1);u32be(&mut v,12,16);v[16..20].copy_from_slice(b"StaA");u32be(&mut v,20,2);u16be(&mut v,36,7);u16be(&mut v,44,9);
    match parse(&v){Data::ResponseCurveSet16(v)=>{assert_eq!(v.response_curve_structures[0].for_each_channel,[2]);assert_eq!(v.response_curve_structures[0].response_arrays.len(),2);assert_eq!(v.response_curve_structures[0].response_arrays[1].encoding_the_interval,9);},x=>panic!("{x:?}")}
    u32be(&mut v,20,u32::MAX);assert!(Data::parse(&v,v.len(),VERSION).is_err());
}
#[test]
fn profile_sequence_uses_embedded_tags_and_advances_records() {
    let mut v=tag("pseq",12);u32be(&mut v,8,2);
    for maker in [0x41424344,0x45464748]{let pos=v.len();v.resize(pos+20,0);u32be(&mut v,pos,maker);v.extend(mluc());v.extend(mluc());}
    match parse(&v){Data::ProfileDescription(v)=>{assert_eq!(v.len(),2);assert_eq!(v[1].device_manufacturer,0x45464748);assert_eq!(v[1].description_device_model.unicode_strings[1].unicode_string,"あ");},x=>panic!("{x:?}")}
    for n in 0..v.len(){assert!(Data::get("pseq",&v,n,VERSION).is_err());}
}

