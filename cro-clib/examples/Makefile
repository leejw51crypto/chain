all: build run
build:
	cargo build
	gcc -g test.c tx.c rpc.c -o cro -lcro_clib -lssl -lm -lcrypto -ldl -lpthread -L../../target/debug/
run:
	LD_LIBRARY_PATH=../../target/debug ./cro

ios: ios_build ios_run
ios_build:
	cargo build
	gcc -g test.c tx.c rpc.c -o cro -lcro_clib -L../../target/debug/
ios_run:
	LD_LIBRARY_PATH=../../target/debug ./cro


