"""Run pinned Rust CMS and local LittleCMS as black boxes; never read their sources."""
import ctypes as C,ctypes.util,sys,struct,itertools,json,hashlib,subprocess
from pathlib import Path
source,destination,prefix=map(Path,sys.argv[1:4]);
assert not any(Path(str(prefix)+'.'+kind+'.bin').exists() for kind in ['icc','mox','lcms']), 'Use a fresh output prefix to avoid stale results'
b=source.read_bytes(); channels=4 if b[16:20]==b'CMYK' else 3
n=9 if channels==4 else 17
values=[round(x*65535/(n-1)) for pixel in itertools.product(range(n),repeat=channels) for x in pixel]
pixels=len(values)//channels
libpath=ctypes.util.find_library('lcms2');lib=C.CDLL(libpath)
def fn(name,ret,args):
 f=getattr(lib,name);f.restype=ret;f.argtypes=args;return f
P=C.c_void_p;U=C.c_uint32
version=fn('cmsGetEncodedCMMversion',U,[])()
assert version==2190, f'Expected LittleCMS 2.19, found {version}'
op=fn('cmsOpenProfileFromMem',P,[P,U]);close=fn('cmsCloseProfile',C.c_int,[P]);create=fn('cmsCreateTransform',P,[P,U,P,U,U,U]);run=fn('cmsDoTransform',None,[P,P,P,U]);delete=fn('cmsDeleteTransform',None,[P])
sbuf=C.create_string_buffer(b);db=destination.read_bytes();dbuf=C.create_string_buffer(db)
s=op(sbuf,len(b));d=op(dbuf,len(db));assert s and d
# Public lcms2 pixel-format macros: colorspace <<16 | channels <<3 | bytes.
fmt=((6 if channels==4 else 4)<<16)|(channels<<3)|2
tr=create(s,fmt,d,(4<<16)|(3<<3)|2,1,0x0140);assert tr,'LittleCMS transform creation failed'
i=(C.c_uint16*len(values))(*values);o=(C.c_uint16*(pixels*3))();run(tr,i,o,pixels)
ref=list(o);Path(str(prefix)+'.lcms.bin').write_bytes(struct.pack('<'+'H'*len(ref),*ref));delete(tr);close(s);close(d)
exe=Path('target/cms-compare/release/icc-cms-compare.exe')
proc=subprocess.run([str(exe),str(source),str(destination),str(prefix)],capture_output=True,text=True);print(proc.stdout);assert proc.returncode==0,proc.stderr
report={'source':source.name,'source_sha256':hashlib.sha256(b).hexdigest(),'destination':destination.name,'destination_sha256':hashlib.sha256(db).hexdigest(),'lcms_version':version,'lcms_binary_sha256':hashlib.sha256(Path(libpath).read_bytes()).hexdigest(),'moxcms_version':'0.9.2','intent':'relative_colorimetric','lcms_flags':'NOOPTIMIZE|NOCACHE (0x0140)','pixels':pixels,'tolerance_codes':1024,'note':'screening tolerance, not a DeltaE accuracy guarantee','status':proc.stdout,'comparison':{}}
for kind in ['icc','mox']:
 path=Path(str(prefix)+'.'+kind+'.bin')
 if not path.exists():continue
 actual=struct.unpack('<'+'H'*len(ref),path.read_bytes());diff=[abs(a-r) for a,r in zip(actual,ref)];pd=[max(diff[k:k+3]) for k in range(0,len(diff),3)]
 report['comparison'][kind]={'max_abs_codes':max(diff),'mean_abs_codes':sum(diff)/len(diff),'max_normalized':max(diff)/65535,'mean_normalized':sum(diff)/len(diff)/65535,'pixels_over_tolerance':sum(x>1024 for x in pd),'exact_match_pixels':sum(x==0 for x in pd),'pixels_over_32_codes':sum(x>32 for x in pd),'pixels_over_256_codes':sum(x>256 for x in pd)}
Path(str(prefix)+'.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2))
