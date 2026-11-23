import math, subprocess, sys, os
S=sys.argv[1]
H=(192,37,192); OLD=(194,37,195)
def at(p): return (p[0]-OLD[0]+H[0], p[1], p[2]-OLD[2]+H[2])
# name: (target, camera offset (dx, dy, dz), time of day)
spots = {
 'hearthstead_plaza': ((194,38,195), (-4,20,30), 0.30),
 'watch_tower': ((200,40,180), (-28,16,10), 0.32),
 'north_watch': ((194,30,144), (24,22,26), 0.30),
 'logging_camp': ((149,34,109), (26,24,24), 0.27),
 'hollow_oak': ((107,27,142), (28,24,22), 0.36),
 'barrow_crypt': ((70,30,205), (22,20,16), 0.62),
 'wrecked_caravan': ((244,31,186), (20,18,22), 0.30),
 'miners_rest': ((322,33,205), (-24,20,22), 0.31),
 'hollow_mines': ((351,30,176), (-26,26,26), 0.34),
}
for name,(t,off,time) in spots.items():
    tx,ty,tz=at(t)
    cx,cy,cz=tx+off[0],ty+off[1],tz+off[2]
    dx,dy,dz=tx-cx,ty-cy,tz-cz
    yaw=math.degrees(math.atan2(-dx,-dz)); pitch=math.degrees(math.atan2(dy,math.hypot(dx,dz)))
    out=f'{S}/concepts/ref/{name}.ppm'
    r=subprocess.run(['./target/debug/dyno','shot','127.0.0.1:7499','--pos',f'{cx},{cy},{cz}','--look',f'{yaw:.1f},{pitch:.1f}','--time',str(time),'--size','1280x720','--out',out],capture_output=True,text=True,timeout=300)
    subprocess.run(['magick',out,'-quality','90',out.replace('.ppm','.jpg')])
    print(name, (r.stdout or r.stderr).strip().splitlines()[-1][:60])
