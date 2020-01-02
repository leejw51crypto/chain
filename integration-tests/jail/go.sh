#!/bin/bash
docker run --rm -it -v $PWD/disk:/root/disk -v /nix:/nix --device /dev/isgx my
