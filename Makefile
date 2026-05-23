# Makefile
.PHONY: build clean run test stress-test

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

stress-test: build
	@echo "Starting stress test pipeline..."
	bash -c ' \
		trap "echo \"Cleaning up...\"; kill 0" EXIT; \
		./target/release/siem & \
		SERVER_PID=$$!; \
		sleep 1; \
		./tools/analytics & \
		ANALYTICS_PID=$$!; \
		sleep 0.5; \
		cargo run --release --bin blaster; \
	'

clean:
	rm -rf target/ tools/forwarder tools/analytics storage/
	cargo clean
