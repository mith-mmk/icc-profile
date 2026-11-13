// macOS SDK public ColorSync API, no image decoder or sips intent mediation.
#include <ColorSync/ColorSync.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static CFDataRef read_data(const char *path) {
 FILE *f=fopen(path,"rb"); if(!f){perror(path);exit(1);} fseek(f,0,SEEK_END); long n=ftell(f); rewind(f);
 unsigned char *b=malloc(n); if(!b || fread(b,1,n,f)!=(size_t)n)exit(1); fclose(f);
 CFDataRef d=CFDataCreate(NULL,b,n);free(b);return d;
}
int main(int argc,char **argv) {
 if(argc!=7){fprintf(stderr,"usage: colorsync SOURCE DEST INPUT OUTPUT INTENT BPC\n");return 1;}
 CFDataRef sd=read_data(argv[1]),dd=read_data(argv[2]),in=read_data(argv[3]);
 if(CFDataGetLength(sd)<128 || CFDataGetLength(dd)<128 || memcmp(CFDataGetBytePtr(sd)+16,"CMYK",4) || memcmp(CFDataGetBytePtr(dd)+16,"RGB ",4)) return 2;
 ColorSyncProfileRef s=ColorSyncProfileCreate(sd,NULL),d=ColorSyncProfileCreate(dd,NULL); if(!s||!d)return 2;
 CFStringRef intents[]={kColorSyncRenderingIntentPerceptual,kColorSyncRenderingIntentRelative,kColorSyncRenderingIntentSaturation,kColorSyncRenderingIntentAbsolute};
 int intent=atoi(argv[5]); if(intent<0||intent>3)return 1;
 const void *keys[]={kColorSyncProfile,kColorSyncRenderingIntent,kColorSyncTransformTag,kColorSyncBlackPointCompensation};
 const void *sv[]={s,intents[intent],kColorSyncTransformDeviceToPCS,atoi(argv[6])?kCFBooleanTrue:kCFBooleanFalse};
 const void *dv[]={d,intents[intent],kColorSyncTransformPCSToDevice,atoi(argv[6])?kCFBooleanTrue:kCFBooleanFalse};
 CFDictionaryRef seqs[]={CFDictionaryCreate(NULL,keys,sv,4,&kCFTypeDictionaryKeyCallBacks,&kCFTypeDictionaryValueCallBacks),CFDictionaryCreate(NULL,keys,dv,4,&kCFTypeDictionaryKeyCallBacks,&kCFTypeDictionaryValueCallBacks)};
 CFArrayRef seq=CFArrayCreate(NULL,(const void **)seqs,2,&kCFTypeArrayCallBacks);
 const void *ok[]={kColorSyncConvertQuality}, *ov[]={kColorSyncBestQuality};
 CFDictionaryRef options=CFDictionaryCreate(NULL,ok,ov,1,&kCFTypeDictionaryKeyCallBacks,&kCFTypeDictionaryValueCallBacks);
 ColorSyncTransformRef t=ColorSyncTransformCreate(seq,options); if(!t)return 3;
 CFTypeRef info=ColorSyncTransformCopyProperty(t,kColorSyncTransformInfo,NULL);if(info){CFShow(info);CFRelease(info);}
 size_t n=CFDataGetLength(in); if(!n || n%4)return 4;size_t pixels=n/4; unsigned char *out=malloc(pixels*3);if(!out)return 4;
 if(!ColorSyncTransformConvert(t,pixels,1,out,kColorSync8BitInteger,kColorSyncAlphaNone,pixels*3,CFDataGetBytePtr(in),kColorSync8BitInteger,kColorSyncAlphaNone,n,NULL))return 5;
 FILE *f=fopen(argv[4],"wb");if(!f||fwrite(out,3,pixels,f)!=pixels)return 6;fclose(f);
 free(out);CFRelease(t);CFRelease(options);CFRelease(seq);CFRelease(seqs[0]);CFRelease(seqs[1]);CFRelease(s);CFRelease(d);CFRelease(sd);CFRelease(dd);CFRelease(in);return 0;
}
