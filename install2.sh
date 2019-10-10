#!/bin/bash
FILE2=/usr/local/bin/kcov
if test -f "$FILE2"; then
    echo "$FILE exist"
    wget https://github.com/SimonKagstrom/kcov/archive/master.tar.gz
    tar xzf master.tar.gz
    cd kcov-master
    mkdir build
    cd build
    cmake ..
    make
    sudo make install
    cd ../..
    rm -rf kcov-master
fi
