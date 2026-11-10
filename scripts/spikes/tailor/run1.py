import time, sys
from lab import *
from fitlab import *
from fit2 import *
B = Body(load_body())
col = body_collider(B)
sp, st = load_glb(SHORTS)
fs, st, info = fit_pants(sp, st, B, col)
print("pants", info)
gp, gn = garment_collider(fs, st)
col2 = Collider(np.concatenate([col.pts, gp]), np.concatenate([col.nrm, gn]))
tp, tt = load_glb(TOP)
ft, tt, info = fit_top(tp, tt, B, col2)
print("top", info)
np.savez(SP+"/fit_shorts.npz", pos=fs, tris=st); np.savez(SP+"/fit_top.npz", pos=ft, tris=tt)
skin=(205,170,150)
box=((-45,-30,50),(45,30,155))
sheet([(B.pos,B.tris,skin),(fs,st,(230,230,235)),(ft,tt,(200,60,60))], SP+"/fit_both.png", box=box, size=560)
sheet([(B.pos,B.tris,skin),(fs,st,(230,230,235))], SP+"/fit_shorts.png", box=box, size=560)

sheet([(B.pos,B.tris,skin),(ft,tt,(200,60,60))], SP+"/top_close.png", box=((-28,-20,100),(28,20,156)), size=560, views=("front","side","back","top"))
