# tools/test_supervisor.exs
# Start the process manually to test supervision logic
{:ok, pid} = SiemSupervisor.RustProcess.start_link([])

IO.puts("SIEM started.")
Process.sleep(2000)

IO.puts("Sending PANIC command to Rust SIEM...")
# Connect to the UDS created by Rust
{:ok, socket} = :gen_tcp.connect({:local, ~c'/tmp/siem_control.sock'}, 0, [:binary, packet: 0])
:gen_tcp.send(socket, "PANIC
")
:gen_tcp.close(socket)

Process.sleep(3000)
IO.puts("Checking if SIEM restarted...")
# If restarting, it should be listening again
case :gen_tcp.connect({:local, ~c'/tmp/siem_control.sock'}, 0, [:binary, packet: 0]) do
    {:ok, socket} -> 
        IO.puts("SIEM successfully restarted!")
        :gen_tcp.close(socket)
    {:error, _} -> 
        IO.puts("SIEM failed to restart!")
end
