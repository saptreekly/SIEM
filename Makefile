# Makefile
.PHONY: build clean run test stress-test stop

build:
	@echo "Building Rust core..."
	cargo build --release
	@echo "Building Zig forwarder..."
	zig build-exe tools/forwarder.zig -O ReleaseFast -femit-bin=tools/forwarder
	@echo "Building Odin analytics..."
	odin build tools/analytics.odin -file -out:tools/analytics -o:speed

run: build
	@echo "Starting SIEM ensemble..."
	./target/release/siem & echo $$! > siem.pid
	./tools/forwarder & echo $$! >> siem.pid
	@echo "SIEM is running in the background. (PIDs stored in siem.pid)"

stop:
	@if [ -f siem.pid ]; then \
		echo "Stopping SIEM ensemble..."; \
		xargs kill < siem.pid; \
		rm siem.pid; \
		echo "SIEM ensemble stopped."; \
	else \
		echo "No SIEM ensemble running."; \
	fi

test:
	cargo test
	./tools/run_analytics.sh

stress-test: build
	@echo "Starting stress test pipeline..."
	@rm -f /tmp/siem_shm.bin;
	@./target/release/siem & \
	SERVER_PID=$$!; \
	echo $$SERVER_PID > test.pid; \
	sleep 1; \
	./tools/analytics & \
	ANALYTICS_PID=$$!; \
	echo $$ANALYTICS_PID >> test.pid; \
	sleep 0.5; \
	cargo run --release --bin blaster; \
	TEST_EXIT_CODE=$$?; \
	echo "Blaster finished with $$TEST_EXIT_CODE. Cleaning up..."; \
	xargs kill < test.pid; \
	rm test.pid; \
	exit $$TEST_EXIT_CODE

clean:
	rm -rf target/ tools/forwarder tools/analytics storage/ siem.pid
	cargo clean
