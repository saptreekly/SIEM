defmodule SiemTest do
  @performer_ip "127.0.0.1"
  @performer_control_port 8081

  def run do
    IO.puts("Simulating interaction with a remote SIEM Performer.")
    IO.puts("Attempting to connect to #{@performer_ip}:#{@performer_control_port}")

    # Give the remote performer some time to start, if it were running separately.
    Process.sleep(2000)

    IO.puts("Sending PANIC command to remote Rust SIEM...")

    case SiemSupervisor.ControlClient.send_command(@performer_ip, @performer_control_port, "PANIC") do
      :ok -> IO.puts("PANIC command sent successfully.")
      {:error, reason} -> IO.puts("Failed to send PANIC command: #{inspect(reason)}")
    end

    Process.sleep(3000)
    IO.puts("Checking if remote SIEM is reachable after potential restart...")

    case SiemSupervisor.ControlClient.check_performer_health(@performer_ip, @performer_control_port) do
      :ok -> IO.puts("Remote SIEM is reachable (potentially restarted)!")
      {:error, _} -> IO.puts("Remote SIEM is not reachable!")
    end
  end
end

# Note: This test supervisor assumes a Rust Performer is already running
# on @performer_ip:@performer_control_port before this script is executed.
# In a full CI/CD, the Rust Performer would be started as part of the test setup.

SiemTest.run()
