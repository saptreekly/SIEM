# Makefile
.PHONY: build clean run test stress-test stop

build:
	@echo "Building Rust core..."
	cargo build --release
	@echo "Building Zig forwarder..."
	zig build-exe tools/forwarder.zig -O ReleaseFast -femit-bin=tools/forwarder -lc
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
	@if [ -f siem.pid ]; then \
		echo "Cleaning up stale PIDs..."; \
		xargs kill < siem.pid 2>/dev/null; \
		rm siem.pid; \
	fi
	@rm -f /tmp/siem_shm.bin;
	@./target/release/siem & \
	SERVER_PID=$$!; \
	echo $$SERVER_PID > siem.pid; \
	sleep 1; \
	./tools/analytics & \
	ANALYTICS_PID=$$!; \
	echo $$ANALYTICS_PID >> siem.pid; \
	sleep 0.5; \
	./target/release/blaster; \
	TEST_EXIT_CODE=$$?; \
	echo "Blaster finished with $$TEST_EXIT_CODE. Cleaning up..."; \
	xargs kill < siem.pid 2>/dev/null; \
	rm siem.pid; \
	exit $$TEST_EXIT_CODE

clean:
	rm -rf target/ tools/forwarder tools/analytics storage/ siem.pid
	cargo clean
