from PIL import Image,ImageCms
from pathlib import Path
import array,sys
p=Path('.test-cms-compare'); im=Image.open(p/'astronaut.png'); assert im.mode=='RGB' and im.size==(512,512)
p.joinpath('astronaut-embedded.icc').write_bytes(im.info['icc_profile'])
a=array.array('H',(x*257 for x in im.tobytes()))
if sys.byteorder!='little': a.byteswap()
p.joinpath('astronaut.rgb16.bin').write_bytes(a.tobytes())
print(ImageCms.getProfileDescription(ImageCms.ImageCmsProfile(str(p/'astronaut-embedded.icc'))))
