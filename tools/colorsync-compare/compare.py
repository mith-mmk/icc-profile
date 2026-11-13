"""Reproduce CMYK8 comparisons using public APIs; original profiles are never modified.
Build the two probes per README.md first. Outputs require a new directory.
"""
import argparse, hashlib, itertools, json, platform, struct, subprocess, zlib
from pathlib import Path


def sha(data):
    return hashlib.sha256(data).hexdigest()


def profile_info(path):
    b = path.read_bytes()
    tags = []
    for i in range(struct.unpack_from('>I', b, 128)[0]):
        sig, off, size = struct.unpack_from('>4sII', b, 132 + 12*i)
        tags.append(dict(signature=sig.decode('ascii'), offset=off, size=size,
                         type=b[off:off+4].decode('ascii'), sha256=sha(b[off:off+size])))
    return dict(name=path.name, sha256=sha(b), bytes=len(b), version=b[8:12].hex(),
                device_class=b[12:16].decode(), color_space=b[16:20].decode(),
                pcs=b[20:24].decode(), header_intent=struct.unpack_from('>I', b, 64)[0], tags=tags)


def tiff(profile, samples):
    # One uncompressed chunky separated CMYK8 row with embedded ICC.
    count = 11
    extra = 8 + 2 + count*12 + 4
    bits = extra
    icc = bits + 8
    pixels = icc + len(profile)
    entries = [(256,4,1,len(samples)//4),(257,4,1,1),(258,3,4,bits),
               (259,3,1,1),(262,3,1,5),(273,4,1,pixels),(277,3,1,4),
               (278,4,1,1),(279,4,1,len(samples)),(284,3,1,1),(34675,7,len(profile),icc)]
    b = b'II'+struct.pack('<HIH',42,8,count)
    for tag,typ,n,v in entries:
        value = struct.pack('<H',v)+b'\0\0' if typ==3 and n==1 else struct.pack('<I',v)
        b += struct.pack('<HHI',tag,typ,n)+value
    return b+struct.pack('<I4H',0,8,8,8,8)+profile+samples


def png_rgb(path):
    b=path.read_bytes(); pos=8; compressed=b''
    assert b[:8]==b'\x89PNG\r\n\x1a\n'
    while pos<len(b):
        n=struct.unpack_from('>I',b,pos)[0];typ=b[pos+4:pos+8];d=b[pos+8:pos+8+n];pos+=12+n
        if typ==b'IHDR': w,h,depth,color,co,fi,inter=struct.unpack('>IIBBBBB',d)
        if typ==b'IDAT': compressed+=d
    assert depth==8 and color in (2,6) and inter==0
    channels={2:3,6:4}[color];size=w*channels;data=zlib.decompress(compressed);prev=[0]*size;out=[];p=0
    for _ in range(h):
        f=data[p];p+=1;row=list(data[p:p+size]);p+=size
        for i in range(size):
            a=row[i-channels] if i>=channels else 0;b=prev[i];c=prev[i-channels] if i>=channels else 0
            if f==1:v=a
            elif f==2:v=b
            elif f==3:v=(a+b)//2
            elif f==4:
                q=a+b-c;ds=[abs(q-a),abs(q-b),abs(q-c)];v=[a,b,c][ds.index(min(ds))]
            else: assert f==0;v=0
            row[i]=(row[i]+v)&255
        out.extend(v for i,v in enumerate(row) if i%channels<3);prev=row
    return bytes(out)


def diff(a,b,inputs):
    assert len(a)==len(b)==len(inputs)//4*3
    ds=[abs(x-y) for x,y in zip(a,b)]
    ranked=sorted(range(len(inputs)//4), key=lambda i:max(ds[i*3:i*3+3]),reverse=True)
    return dict(max_codes=max(ds),mean_codes=sum(ds)/len(ds),channels_over_2=sum(d>2 for d in ds),
                first_sha256=sha(a),second_sha256=sha(b),
                worst=[dict(index=i,cmyk=list(inputs[i*4:i*4+4]),first=list(a[i*3:i*3+3]),
                            second=list(b[i*3:i*3+3])) for i in ranked[:3]])


def run(cmd,log):
    p=subprocess.run(list(map(str,cmd)),capture_output=True,text=True)
    log.write_text(p.stdout+p.stderr)
    return dict(returncode=p.returncode,log=p.stdout+p.stderr)


def main():
    ap=argparse.ArgumentParser();ap.add_argument('source',type=Path);ap.add_argument('destination',type=Path)
    ap.add_argument('output',type=Path);ap.add_argument('--icc-bin',required=True,type=Path)
    ap.add_argument('--colorsync-bin',required=True,type=Path);a=ap.parse_args()
    assert a.source.read_bytes()[16:20] == b'CMYK'
    assert a.destination.read_bytes()[16:20] == b'RGB '
    a.output.mkdir(parents=True,exist_ok=False)
    report=dict(platform=platform.platform(),baseline_commit='df5ff0dde65144104a5a469595e3e36c4bd24907',profiles=[profile_info(a.source),profile_info(a.destination)],
                configuration=dict(icc_bpc=False,colorsync_bpc='explicit false (main); true (diagnostic)',
                                   colorsync_quality='kColorSyncBestQuality',colorsync_cmm='system default; not pinned',
                                   colorsync_intent='same explicit intent on both profile sequence entries',
                                   directions=['kColorSyncTransformDeviceToPCS','kColorSyncTransformPCSToDevice'],
                                   format='chunky CMYK8 to RGB8, no alpha',icc_interpolation='default tetrahedral'), grids={})
    intents=['perceptual','relative','saturation','absolute']
    for name,levels in [('625',[0,63,127,191,255]),('6561',[0,32,64,96,128,159,191,223,255])]:
        root=a.output/name;root.mkdir()
        # C fastest, then M, Y, K, matching the historical 625 swatches.
        inputs=bytes(v for p in itertools.product(levels,repeat=4) for v in reversed(p))
        inp=root/'input.cmyk';inp.write_bytes(inputs)
        data={};statuses={}
        for i,intent in enumerate(intents):
            key=f'icc-{intent}';out=root/(key+'.rgb')
            statuses[key]=run([a.icc_bin,a.source,a.destination,inp,out,i],root/(key+'.log'))
            if statuses[key]['returncode']==0:data[key]=out.read_bytes()
            for bpc in (0,1):
                key=f'cs-{intent}-bpc{bpc}';out=root/(key+'.rgb')
                statuses[key]=run([a.colorsync_bin,a.source,a.destination,inp,out,i,bpc],root/(key+'.log'))
                if statuses[key]['returncode']==0:data[key]=out.read_bytes()
        comparisons={}
        for intent in intents:
            key=f'icc-{intent}';ref=f'cs-{intent}-bpc0'
            if key in data and ref in data:comparisons[f'{key}_vs_{ref}']=diff(data[key],data[ref],inputs)
            comparisons[f'cs-{intent}-bpc0_vs_bpc1']=diff(data[ref],data[f'cs-{intent}-bpc1'],inputs)
        if name=='625':
            tif=root/'swatches.tif';tif.write_bytes(tiff(a.source.read_bytes(),inputs))
            for intent in intents:
                key=f'sips-{intent}';out=root/(key+'.png')
                statuses[key]=run(['sips','-M',a.destination,intent,'-s','format','png','--out',out,tif],root/(key+'.log'))
                if statuses[key]['returncode']==0:
                    data[key]=png_rgb(out);(root/(key+'.rgb')).write_bytes(data[key])
                    for target in [f'icc-{intent}','icc-relative',f'cs-{intent}-bpc0','cs-relative-bpc0']:
                        if target in data:comparisons[f'{key}_vs_{target}']=diff(data[key],data[target],inputs)
        report['grids'][name]=dict(levels=levels,input_sha256=sha(inputs),statuses=statuses,
                                  output_sha256={k:sha(v) for k,v in data.items()},comparisons=comparisons)
    # Isolate header intent from the sips command-line intent. Original bytes stay intact.
    root=a.output/'header-control';root.mkdir()
    inputs=(a.output/'625/input.cmyk').read_bytes()
    controls={}
    for header in range(4):
        b=bytearray(a.source.read_bytes());struct.pack_into('>I',b,64,header)
        tif=root/f'header-{header}.tif';tif.write_bytes(tiff(b,inputs))
        controls[str(header)]={}
        for intent in intents:
            out=root/f'header-{header}-{intent}.png'
            status=run(['sips','-M',a.destination,intent,'-s','format','png','--out',out,tif],root/f'header-{header}-{intent}.log')
            assert status['returncode']==0,status
            data=png_rgb(out)
            entry=dict(output_sha256=sha(data),vs_original_sips=diff(data,(a.output/'625/sips-relative.rgb').read_bytes(),inputs),
                       vs_colorsync_header_intent=diff(data,(a.output/f'625/cs-{intents[header]}-bpc0.rgb').read_bytes(),inputs))
            target=a.output/f'625/icc-{intents[header]}.rgb'
            if target.exists():entry['vs_icc_header_intent']=diff(data,target.read_bytes(),inputs)
            controls[str(header)][intent]=entry
    report['header_control']=controls
    # Keep published provenance portable and avoid machine-local paths in logs.
    serialized=json.dumps(report,indent=2)+'\n'
    for path,label in [(a.output.resolve(),'<OUTPUT>'),(a.source.resolve(),'<SOURCE>'),(a.destination.resolve(),'<DESTINATION>')]:
        serialized=serialized.replace(str(path),label)
    (a.output/'report.json').write_text(serialized)
    for grid,g in report['grids'].items():
        for key,d in g['comparisons'].items(): print(grid,key,d['max_codes'],round(d['mean_codes'],6))

if __name__=='__main__':main()
