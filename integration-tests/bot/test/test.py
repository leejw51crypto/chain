#!/usr/bin/env python3
from chainrpc import RPC
import time

rpc = RPC()
PASSWD = "27243138a="
print("test")

#rpc.wallet.restore("annual dinosaur deliver hour loop food buddy lift alert obvious thank scorpion young amused climb defy erode blur drip gun require clerk beef armed ", "a", PASSWD)

def test():
    count=0 
    while True :
        rpc.address.create("a", "transfer", "e27735ed63826a3e419e17b97ccc7bb488f141a2f469a865d646193b1d385aed")
        count = count + 1
        print("{0}".format(count))
        time.sleep(0.001)

rpc.address.create("a", "transfer", "e27735ed63826a3e419e17b97ccc7bb488f141a2f469a865d646193b1d385aed")
test()
