# Test script for RustProcess
require Logger
Logger.configure(level: :debug)

# Start application (assuming this is how it starts)
# Based on application.ex it should be a supervisor
{:ok, pid} = SiemSupervisor.RustProcess.start_link(node_id: "test-node", name: :rust_process_test)

IO.puts("Supervisor started, PID: #{inspect(pid)}")

# Wait for process to start
Process.sleep(2000)

# Check if port is running
{os_pid, _} = System.cmd("pgrep", ["-f", "target/release/siem"])
IO.puts("Rust process PID: #{os_pid}")

# Kill the process
{pid_to_kill, _} = System.cmd("pgrep", ["-f", "target/release/siem"])
System.cmd("kill", ["-9", String.trim(pid_to_kill)])
IO.puts("Killed Rust process")

# Wait for restart
Process.sleep(2000)

# Verify restart
{new_pid, _} = System.cmd("pgrep", ["-f", "target/release/siem"])
IO.puts("New Rust process PID: #{new_pid}")

if new_pid != pid_to_kill do
  IO.puts("Test Passed: Process restarted successfully.")
else
  IO.puts("Test Failed: Process did not restart.")
end

# Clean up
GenServer.stop(:rust_process_test)
