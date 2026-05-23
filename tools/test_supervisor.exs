defmodule SiemTest do
  def find_socket do
    sockets = Path.wildcard("/tmp/siem_control_performer_*.sock")
    case sockets do
      [path | _] -> String.to_charlist(path)
      [] -> nil
    end
  end

  def run do
    # Start the process manually to test supervision logic
    {:ok, pid} = SiemSupervisor.RustProcess.start_link([])

    IO.puts("SIEM started.")
    Process.sleep(2000)

    IO.puts("Sending PANIC command to Rust SIEM...")
    
    socket_path = find_socket()
    if socket_path do
      {:ok, socket} = :gen_tcp.connect({:local, socket_path}, 0, [:binary, packet: 0])
      :gen_tcp.send(socket, "PANIC\n")
      :gen_tcp.close(socket)
    end

    Process.sleep(3000)
    IO.puts("Checking if SIEM restarted...")
    
    socket_path = find_socket()
    case :gen_tcp.connect({:local, socket_path}, 0, [:binary, packet: 0]) do
        {:ok, socket} -> 
            IO.puts("SIEM successfully restarted!")
            :gen_tcp.close(socket)
        {:error, _} -> 
            IO.puts("SIEM failed to restart!")
    end
  end
end

SiemTest.run()
