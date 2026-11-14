"""Public LCMS API only: compare CMYK files in one predicted-print Lab space.
No LCMS implementation source is read. No repository implementation is changed.
"""
import ctypes as C, ctypes.util, json, math, pathlib, hashlib
root=pathlib.Path(__file__).resolve().parents[2]
out=root/'.test-cms-compare'
dll=ctypes.util.find_library('lcms2'); lib=C.CDLL(dll)
class Lab(C.Structure): _fields_=[('L',C.c_double),('a',C.c_double),('b',C.c_double)]
def api(name,restype,args):
 f=getattr(lib,name); f.restype=restype; f.argtypes=args; return f
ptr=C.c_void_p; u=C.c_uint32
openp=api('cmsOpenProfileFromFile',ptr,[C.c_char_p,C.c_char_p])
labp=api('cmsCreateLab4Profile',ptr,[ptr])
create=api('cmsCreateTransform',ptr,[ptr,u,ptr,u,u,u])
do=api('cmsDoTransform',None,[ptr,ptr,ptr,u])
de00=api('cmsCIE2000DeltaE',C.c_double,[C.POINTER(Lab),C.POINTER(Lab),C.c_double,C.c_double,C.c_double])
version=api('cmsGetEncodedCMMversion',C.c_int,[])()
assert version==2190, 'Expected LittleCMS 2.19'
profile=root/'_test_samples/CGATS21_CRPC6.icc'
p=openp(str(profile).encode(),'r'.encode()); q=labp(None)
# Public lcms2.h encodings: PT_CMYK=6, PT_Lab=10, FLOAT_SH(1)=1<<22.
t=create(p,(6<<16)|(4<<3)|2,q,(1<<22)|(10<<16)|(3<<3),1,0x140)
assert t and p and q
n=17**3; data={}; labs={}; hashes={}
for bpc,prefix in [('off','output-xrite'),('on','output-bpc-final-xrite')]:
 for interp in ['trilinear','tetrahedral']:
  for engine in ['icc','lcms']:
   key=f'{bpc}/{interp}/{engine}'
   f=out/f'{prefix}-{interp}.{engine}.bin'; raw=f.read_bytes(); assert len(raw)==n*8
   data[key]=(C.c_uint16*(n*4)).from_buffer_copy(raw); labs[key]=(Lab*n)()
   do(t,data[key],labs[key],n); hashes[str(f.name)]=hashlib.sha256(raw).hexdigest()
def quant(xs,p):
 a=sorted(xs); pos=(len(a)-1)*p; lo=int(pos); return a[lo]+(a[min(lo+1,len(a)-1)]-a[lo])*(pos-lo)
def stat(xs): return dict(mean=sum(xs)/len(xs),max=max(xs),p95=quant(xs,.95),p99=quant(xs,.99),over1=sum(x>1 for x in xs),over2=sum(x>2 for x in xs),over3=sum(x>3 for x in xs))
def labv(x): return [x.L,x.a,x.b]
def rgb(i): return [int(math.floor(x*65535/16+.5)) for x in [i//289,(i//17)%17,i%17]]
def compare(a,b):
 ds=[]; d76=[]; code=[]
 for i,(x,y) in enumerate(zip(labs[a],labs[b])):
  ds.append(de00(C.byref(x),C.byref(y),1,1,1)); d76.append(math.dist(labv(x),labv(y)))
  code.append(max(abs(data[a][4*i+j]-data[b][4*i+j]) for j in range(4)))
 worst=[]
 for i in sorted(range(n),key=lambda i:ds[i],reverse=True)[:5]:
  worst.append(dict(index=i,rgb16=rgb(i),deltaE00=ds[i],deltaE76=d76[i],cmykA=list(data[a][4*i:4*i+4]),cmykB=list(data[b][4*i:4*i+4]),labA=labv(labs[a][i]),labB=labv(labs[b][i])))
 i=max(range(n),key=lambda i:code[i])
 return dict(a=a,b=b,deltaE00=stat(ds),deltaE76=stat(d76),worst=worst,maxCodeDifference=dict(index=i,rgb16=rgb(i),codes=code[i],deltaE00=ds[i],deltaE76=d76[i]))
results={}
for b in ['off','on']:
 for interp in ['trilinear','tetrahedral']: results[f'icc_vs_lcms/{b}/{interp}']=compare(f'{b}/{interp}/icc',f'{b}/{interp}/lcms')
 results[f'icc_tri_vs_tetra/{b}']=compare(f'{b}/trilinear/icc',f'{b}/tetrahedral/icc')
for engine in ['icc','lcms']:
 for interp in ['trilinear','tetrahedral']: results[f'bpc_off_vs_on/{engine}/{interp}']=compare(f'off/{interp}/{engine}',f'on/{interp}/{engine}')
report=dict(method='Same LCMS CMYK16 to Lab double transform, relative colorimetric A2B1, NOOPTIMIZE|NOCACHE, no BPC on evaluation leg. Profile-predicted print Lab, not measured print or display-pixel differences. Quantiles linearly interpolated.',lcms_version=version,lcms_dll=dll,profile_sha256=hashlib.sha256(profile.read_bytes()).hexdigest(),samples=n,input_hashes=hashes,results=results)
(out/'output-xrite-perceptual-error.json').write_text(json.dumps(report,indent=2))
print('LCMS',version)
for k,v in results.items(): print(k,json.dumps(v['deltaE00']), 'max-code',v['maxCodeDifference'])
api('cmsDeleteTransform',None,[ptr])(t)
api('cmsCloseProfile',C.c_int,[ptr])(p); api('cmsCloseProfile',C.c_int,[ptr])(q)
