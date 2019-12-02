#!/bin/bash
service ssh start
source /root/disk/go_common.sh
cd /root/disk
cp ./config0/* /root/.tendermint/config
/root/disk/launch.sh
sleep infinity
