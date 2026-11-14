use std::fs;
fn main() {
 let base=std::path::Path::new(".test-cms-compare");
 let sb=fs::read(base.join("astronaut-embedded.icc")).unwrap();
 let db=fs::read("_test_samples/CGATS21_CRPC6.icc").unwrap();
 let s=icc_profile::Profile::parse(&sb).unwrap(); let d=icc_profile::Profile::parse(&db).unwrap();
 let raw=fs::read(base.join("astronaut.rgb16.bin")).unwrap();
 let input:Vec<u16>=raw.chunks_exact(2).map(|x|u16::from_le_bytes([x[0],x[1]])).collect();
 for bpc in [false,true] { for tetra in [false,true] {
 let opts=icc_profile::TransformOptions { rendering_intent:icc_profile::RenderingIntent::RelativeColorimetric, black_point_compensation:bpc, ..Default::default() };
 let t=icc_profile::Transform::new(&s,&d,opts).unwrap().with_algorithm_profile(icc_profile::AlgorithmProfile::default().with_clut_3d(if tetra {icc_profile::ClutInterpolation3D::Tetrahedral} else {icc_profile::ClutInterpolation3D::Trilinear}));
 let mut out=vec![0u16;input.len()/3*4]; t.transform_u16_atomic(&input,&mut out).unwrap();
 let raw:Vec<u8>=out.iter().flat_map(|v|v.to_le_bytes()).collect();
 let name=format!("photo-{}-{}.icc.bin",if bpc {"on"} else {"off"},if tetra {"tetrahedral"} else {"trilinear"});
 fs::write(base.join(&name),raw).unwrap(); println!("{} {} pixels",name,out.len()/4);
 }}
}
