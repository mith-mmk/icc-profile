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
from PIL import Image
im=Image.open(out/'astronaut.png'); rgbbytes=im.tobytes()
n=im.width*im.height; data={}; labs={}; hashes={}
source=openp(str(out/'astronaut-embedded.icc').encode(),b'r')
input_raw=(out/'astronaut.rgb16.bin').read_bytes()
input16=(C.c_uint16*(n*3)).from_buffer_copy(input_raw)
for bpc in ['off','on']:
 tr=create(source,(4<<16)|(3<<3)|2,p,(6<<16)|(4<<3)|2,1,0x140|(0x2000 if bpc=='on' else 0))
 assert tr
 cm=(C.c_uint16*(n*4))(); do(tr,input16,cm,n)
 for interp in ['trilinear','tetrahedral']:
  (out/f'photo-{bpc}-{interp}.lcms.bin').write_bytes(bytes(cm))
 api('cmsDeleteTransform',None,[ptr])(tr)
api('cmsCloseProfile',C.c_int,[ptr])(source)

for bpc,prefix in [('off','photo-off'),('on','photo-on')]:
 for interp in ['trilinear','tetrahedral']:
  for engine in ['icc','lcms']:
   key=f'{bpc}/{interp}/{engine}'
   f=out/f'{prefix}-{interp}.{engine}.bin'; raw=f.read_bytes(); assert len(raw)==n*8
   data[key]=(C.c_uint16*(n*4)).from_buffer_copy(raw); labs[key]=(Lab*n)()
   do(t,data[key],labs[key],n); hashes[str(f.name)]=hashlib.sha256(raw).hexdigest()
def quant(xs,p):
 a=sorted(xs); pos=(len(a)-1)*p; lo=int(pos); return a[lo]+(a[min(lo+1,len(a)-1)]-a[lo])*(pos-lo)
def stat(xs): return dict(mean=sum(xs)/len(xs),max=max(xs),p95=quant(xs,.95),p99=quant(xs,.99),over1=sum(x>1 for x in xs),over2=sum(x>2 for x in xs),over3=sum(x>3 for x in xs),fractionOver1=sum(x>1 for x in xs)/len(xs),fractionOver2=sum(x>2 for x in xs)/len(xs))
def labv(x): return [x.L,x.a,x.b]
def rgb(i): return [x*257 for x in rgbbytes[3*i:3*i+3]]
def compare(a,b):
 ds=[]; d76=[]; code=[]
 for i,(x,y) in enumerate(zip(labs[a],labs[b])):
  ds.append(de00(C.byref(x),C.byref(y),1,1,1)); d76.append(math.dist(labv(x),labv(y)))
  code.append(max(abs(data[a][4*i+j]-data[b][4*i+j]) for j in range(4)))
 worst=[]
 for i in sorted(range(n),key=lambda i:ds[i],reverse=True)[:5]:
  worst.append(dict(index=i,xy=[i%im.width,i//im.width],rgb8=list(rgbbytes[3*i:3*i+3]),rgb16=rgb(i),deltaE00=ds[i],deltaE76=d76[i],cmykA=list(data[a][4*i:4*i+4]),cmykB=list(data[b][4*i:4*i+4]),labA=labv(labs[a][i]),labB=labv(labs[b][i])))
 i=max(range(n),key=lambda i:code[i])
 return dict(a=a,b=b,deltaE00=stat(ds),deltaE76=stat(d76),worst=worst,maxCodeDifference=dict(index=i,xy=[i%im.width,i//im.width],rgb8=list(rgbbytes[3*i:3*i+3]),rgb16=rgb(i),codes=code[i],deltaE00=ds[i],deltaE76=d76[i]))
results={}
for b in ['off','on']:
 for interp in ['trilinear','tetrahedral']: results[f'icc_vs_lcms/{b}/{interp}']=compare(f'{b}/{interp}/icc',f'{b}/{interp}/lcms')
 results[f'icc_tri_vs_tetra/{b}']=compare(f'{b}/trilinear/icc',f'{b}/tetrahedral/icc')
for engine in ['icc','lcms']:
 for interp in ['trilinear','tetrahedral']: results[f'bpc_off_vs_on/{engine}/{interp}']=compare(f'off/{interp}/{engine}',f'on/{interp}/{engine}')
report=dict(method='Same LCMS CMYK16 to Lab double transform, relative colorimetric A2B1, NOOPTIMIZE|NOCACHE, no BPC on evaluation leg. Profile-predicted print Lab, not measured print or display-pixel differences. Quantiles linearly interpolated.',lcms_version=version,lcms_dll=dll,profile_sha256=hashlib.sha256(profile.read_bytes()).hexdigest(),samples=n,input_hashes=hashes,results=results)
report.update(photo_sha256=hashlib.sha256((out/'astronaut.png').read_bytes()).hexdigest(),source_profile_sha256=hashlib.sha256((out/'astronaut-embedded.icc').read_bytes()).hexdigest(),source_profile='Embedded HP1998 sRGB IEC61966-2.1, not assigned sRGB2014',photo_dimensions=list(im.size),sampling='All 262144 original pixels, byte to u16 times 257, no resize',provenance='https://scikit-image.org/docs/stable/api/skimage.data#skimage.data.astronaut ; extracted image only from official scikit-image 0.25.2 PyPI wheel, NASA photograph, public domain per dataset documentation')
(out/'photo-xrite-perceptual-error.json').write_text(json.dumps(report,indent=2))
# Scientific visualization only: predicted relative colorimetric Lab mapped to sRGB.
srgb=api('cmsCreate_sRGBProfile',ptr,[])()
view=create(q,(1<<22)|(10<<16)|(3<<3),srgb,(4<<16)|(3<<3)|1,1,0x140)
from PIL import ImageDraw
panel=Image.new('RGB',(1536,548),'white'); draw=ImageDraw.Draw(panel)
for col,(key,label) in enumerate([('on/trilinear/icc','ICC trilinear'),('on/tetrahedral/icc','ICC tetrahedral'),('on/tetrahedral/lcms','LCMS')]):
 rgbout=(C.c_uint8*(n*3))(); do(view,labs[key],rgbout,n)
 pic=Image.frombytes('RGB',im.size,bytes(rgbout)); panel.paste(pic,(col*512,36)); draw.text((col*512+12,10),label+' | BPC on | predicted print',fill='black')
# Embed sRGB profile via Pillow only for portable display metadata.
from PIL import ImageCms
panel.save(out/'photo-predicted-bpc-on-comparison.png',icc_profile=ImageCms.ImageCmsProfile(ImageCms.createProfile('sRGB')).tobytes())
api('cmsDeleteTransform',None,[ptr])(view); api('cmsCloseProfile',C.c_int,[ptr])(srgb)

print('LCMS',version)
for k,v in results.items(): print(k,json.dumps(v['deltaE00']), 'max-code',v['maxCodeDifference'])
api('cmsDeleteTransform',None,[ptr])(t)
api('cmsCloseProfile',C.c_int,[ptr])(p); api('cmsCloseProfile',C.c_int,[ptr])(q)
