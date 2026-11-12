from pathlib import Path
import ctypes as C, ctypes.util, hashlib, json
libpath=ctypes.util.find_library('lcms2');lib=C.CDLL(libpath)
P=C.c_void_p;U=C.c_uint32
class XYZ(C.Structure): _fields_=[('X',C.c_double),('Y',C.c_double),('Z',C.c_double)]
def fn(n,r,a):
 f=getattr(lib,n);f.restype=r;f.argtypes=a;return f
op=fn('cmsOpenProfileFromMem',P,[P,U]);close=fn('cmsCloseProfile',C.c_int,[P]);detect=fn('cmsDetectDestinationBlackPoint',C.c_int,[C.POINTER(XYZ),P,U,U]);version=fn('cmsGetEncodedCMMversion',U,[])()
report={'lcms_version':version,'lcms_binary_sha256':hashlib.sha256(Path(libpath).read_bytes()).hexdigest(),'api':'cmsDetectDestinationBlackPoint','intent':1,'flags':0,'profiles':[]}
for name in ['CGATS21_CRPC6.icc','PSOcoated_v3.icc','sRGB2014.icc']:
 b=(Path('_test_samples')/name).read_bytes();buf=C.create_string_buffer(b);p=op(buf,len(b));assert p
 xyz=XYZ();ok=detect(C.byref(xyz),p,1,0);close(p)
 report['profiles'].append({'file':name,'sha256':hashlib.sha256(b).hexdigest(),'ok':ok,'xyz':[xyz.X,xyz.Y,xyz.Z]})
path=Path('.test-cms-compare/output-bpc-public-blackpoints.json')
with path.open('x') as f:json.dump(report,f,indent=2)
print(json.dumps(report,indent=2))
