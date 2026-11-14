"""Independent ICC draft 4.2.5 estimator over public CMS transforms, no CMS sources."""
import ctypes as C,ctypes.util,json,math
from pathlib import Path
lib=C.CDLL(ctypes.util.find_library('lcms2'));P=C.c_void_p;U=C.c_uint32
def fn(n,r,a):
 f=getattr(lib,n);f.restype=r;f.argtypes=a;return f
op=fn('cmsOpenProfileFromMem',P,[P,U]);close=fn('cmsCloseProfile',C.c_int,[P]);lab=fn('cmsCreateLab4Profile',P,[P]);create=fn('cmsCreateTransform',P,[P,U,P,U,U,U]);run=fn('cmsDoTransform',None,[P,P,P,U]);delete=fn('cmsDeleteTransform',None,[P])
LAB=(1<<22)|(10<<16)|(3<<3);CMYK=(1<<22)|(6<<16)|(4<<3)
def call(t,x,n):
 src=(C.c_double*len(x))(*x);dst=(C.c_double*n)();run(t,src,dst,1);return list(dst)
def fit(xs,ys):
 a=[[sum(x**(i+j) for x in xs) for j in range(3)]+[sum(y*x**i for x,y in zip(xs,ys))] for i in range(3)]
 for i in range(3):
  p=max(range(i,3),key=lambda k:abs(a[k][i]));a[i],a[p]=a[p],a[i];v=a[i][i]
  if abs(v)<1e-14:raise ValueError('singular')
  a[i]=[v0/v for v0 in a[i]]
  for j in range(3):
   if j!=i:
    v=a[j][i];a[j]=[r-v*s for r,s in zip(a[j],a[i])]
 return [a[i][3] for i in range(3)]
report=[]
for name in ['CGATS21_CRPC6.icc','PSOcoated_v3.icc']:
 data=(Path('_test_samples')/name).read_bytes();buf=C.create_string_buffer(data);p=op(buf,len(data));l=lab(None)
 ts=[create(l,LAB,p,CMYK,0,0x0140),create(l,LAB,p,CMYK,1,0x0140),create(p,CMYK,l,LAB,1,0x0140)];assert all(ts)
 device=call(ts[0],[0,0,0],4);initial=call(ts[2],device,3);initial[0]=min(50,initial[0]);initial[1:]=[0,0]
 xs=[100*i/255 for i in range(256)];ys=[call(ts[2],call(ts[1],[x,0,0],4),3)[0] for x in xs]
 for i in range(254,-1,-1):ys[i]=min(ys[i],ys[i+1])
 mn,mx=ys[0],ys[-1];straight=mx>mn and all(abs(x-y)<4 for x,y in zip(xs,ys) if x>mn+.2*(mx-mn))
 coeff=None;count=0
 if mx<=mn:z=0;method='invalid'
 elif straight:z=initial[0];method='initial'
 else:
  points=[(x,(y-mn)/(mx-mn)) for x,y in zip(xs,ys) if .1<=(y-mn)/(mx-mn)<.5];count=len(points);method='fit'
  if count<3:z=0
  else:
   c,b,a=fit(*zip(*points));coeff=[a,b,c];d=b*b-4*a*c
   z=max(0,min(50,-c/b)) if abs(a)<1e-10 else (max(0,min(50,(-b+math.sqrt(d))/(2*a))) if d>0 else 0)
 y=((z+16)/116)**3 if z>8 else z/(24389/27)
 report.append(dict(profile=name,initial_lab=initial,device_black_percent=device,ramp_range=[mn,mx],method=method,shadow_points=count,coefficients=coeff,estimated_lstar=z,estimated_y=y))
 for t in ts:delete(t)
 close(l);close(p)
with Path('.test-cms-compare/output-bpc-independent-ramp.json').open('x') as f:json.dump(report,f,indent=2)
print(json.dumps(report,indent=2))
