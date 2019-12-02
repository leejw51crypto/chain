#!/usr/bin/env python3
from chainbot import CLI
import json

a = CLI()
print("make config")
cfg=a.gen(2)
cfg["nodes"][0]["bonded_coin"]=  3750000000000000000
cfg["nodes"][0]["unbonded_coin"]=1250000000000000000
cfg["nodes"][1]["bonded_coin"]=  1250000000000000000
cfg["nodes"][1]["unbonded_coin"]=3750000000000000000
print(json.dumps(cfg, indent=4))
a.prepare_cfg(cfg)