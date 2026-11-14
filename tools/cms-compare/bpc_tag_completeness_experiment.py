"""Original-fixture tag-admission experiment; public LCMS API only."""
from pathlib import Path
import ctypes as C, ctypes.util,struct,json,hashlib,sys
root=Path(sys.argv[1] if len(sys.argv)>1 else '.test-cms-compare');U=lambda b,o:struct.unpack_from('>I',b,o)[0]
def read_tags(b):
 return {b[132+i*12:136+i*12]:b[U(b,136+i*12):U(b,136+i*12)+U(b,140+i*12)] for i in range(U(b,128))}
def pack(base,tags):
 b=bytearray(base[:128])+struct.pack('>I',len(tags))+bytearray(12*len(tags))
 for i,(sig,v) in enumerate(tags.items()):
  b.extend(bytes((-len(b))%4));off=len(b);b[132+i*12:144+i*12]=sig+struct.pack('>II',off,len(v));b.extend(v)
 struct.pack_into('>I',b,0,len(b));return bytes(b)
def mluc(text):
 v=text.encode('utf-16-be');return b'mluc'+bytes(4)+struct.pack('>II',1,12)+b'enUS'+struct.pack('>II',len(v),28)+v
lib=C.CDLL(ctypes.util.find_library('lcms2'));P=C.c_void_p;N=C.c_uint32
class XYZ(C.Structure):_fields_=[('X',C.c_double),('Y',C.c_double),('Z',C.c_double)]
def fn(n,r,a):f=getattr(lib,n);f.restype=r;f.argtypes=a;return f
op=fn('cmsOpenProfileFromMem',P,[P,N]);close=fn('cmsCloseProfile',C.c_int,[P]);detect=fn('cmsDetectBlackPoint',C.c_int,[C.POINTER(XYZ),P,N,N]);support=fn('cmsIsIntentSupported',C.c_int,[P,N,N]);version=fn('cmsGetEncodedCMMversion',N,[])()
report={'lcms_version':version,'profiles':[]}
for pcs in ['lab','xyz']:
 path=root/f'bpc-original-cmyk-{pcs}.icc';original=path.read_bytes();tags=read_tags(original)
 for label in ['baseline','add-a2b0','add-b2a1','all-six','all-six-metadata']:
  t=tags.copy()
  if label in ['add-a2b0','all-six','all-six-metadata']:t[b'A2B0']=tags[b'A2B1']
  if label in ['add-b2a1','all-six','all-six-metadata']:t[b'B2A1']=tags[b'B2A0']
  if label.startswith('all-six'):t[b'A2B2']=tags[b'A2B1'];t[b'B2A2']=tags[b'B2A0']
  if label=='all-six-metadata':
   t[b'wtpt']=b'XYZ '+bytes(4)+struct.pack('>iii',63190,65536,54061);t[b'desc']=mluc('Original BPC tag completeness experiment');t[b'cprt']=mluc('Original synthetic test fixture')
  b=pack(original,t);assert read_tags(b)[b'A2B1']==tags[b'A2B1'];assert read_tags(b)[b'B2A0']==tags[b'B2A0']
  file=root/f'bpc-tag-experiment-{pcs}-{label}.icc'
  with file.open('xb') as f:f.write(b)
  buffer=C.create_string_buffer(b);p=op(buffer,len(b));assert p
  xyz=XYZ();ok=detect(C.byref(xyz),p,1,0)
  item={'pcs':pcs,'variant':label,'file':file.name,'sha256':hashlib.sha256(b).hexdigest(),'detect_ok':ok,'relative_black_xyz':[xyz.X,xyz.Y,xyz.Z],'perceptual_input_supported':support(p,0,0),'perceptual_output_supported':support(p,0,1),'relative_input_supported':support(p,1,0),'relative_output_supported':support(p,1,1)}
  close(p);report['profiles'].append(item);print(json.dumps(item))
 assert path.read_bytes()==original
with (root/'bpc-tag-experiment-results.json').open('x') as f:json.dump(report,f,indent=2)

