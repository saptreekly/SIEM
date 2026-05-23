# Makefile
.PHONY: build clean run test

build:
	@echo "Building Rust core..."
	cargo build --release
	@echo "Building Zig forwarder..."
	zig build-exe tools/forwarder.zig -O ReleaseFast -femit-bin=tools/forwarder
	@echo "Building Odin analytics..."
	odin build tools/analytics.odin -file -out:tools/analytics -o:speed

run: build
	@echo "Starting SIEM ensemble..."
	./target/release/siem & 
	./tools/forwarder &
	@echo "SIEM is running in the background."

test:
	cargo test
	./tools/run_analytics.sh

clean:
	rm -rf target/ tools/forwarder tools/analytics storage/
	cargo clean
