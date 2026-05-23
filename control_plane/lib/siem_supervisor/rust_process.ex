defmodule SiemSupervisor.RustProcess do
  use GenServer
  require Logger

  @check_interval 5000 # 5 seconds

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  @impl true
  def init(:ok) do
    Process.flag(:trap_exit, true)
    executable = Path.join(File.cwd!(), "target/release/siem")
    # Start the port and get the port reference
    port_ref = start_port(executable)

    # Get the OS PID from the port reference
    # :erlang.port_info/2 returns {:os_pid, pid} or {:error, reason}
    os_pid_info = :erlang.port_info(port_ref, :os_pid)

    case os_pid_info do
      {:os_pid, os_pid} ->
        # Calculate the dynamic socket path
        socket_path = "/tmp/siem_control_performer_#{os_pid}.sock"
        Logger.info("Rust SIEM started with OS PID: #{os_pid}, UDS: #{socket_path}")
        # Schedule periodic health check with the socket path
        Process.send_after(self(), {:check_health, socket_path}, @check_interval)
        {:ok, %{port: port_ref, executable: executable, os_pid: os_pid, socket_path: socket_path}}
      {:error, reason} ->
        Logger.error("Failed to get OS PID for Rust SIEM: #{inspect(reason)}. Shutting down.")
        # Consider a more graceful shutdown or restart strategy here
        {:stop, {:init_error, reason}}
    end
  end

  defp start_port(executable) do
    Logger.info("Spawning Rust SIEM executable: #{executable}")
    # Port.open returns a port reference
    Port.open({:spawn, executable}, [:binary, :exit_status, :stderr_to_stdout])
  end

  @impl true
  def handle_info({_port, {:data, data}}, state) do
    Logger.debug("Rust SIEM output: #{data}")
    {:noreply, state}
  end

  @impl true
  def handle_info({_port, {:exit_status, status}}, state) do
    Logger.error("Rust SIEM exited with status #{status}. Restarting...")
    # Restart the process by re-initializing
    # This will re-run init logic to get new port and PID
    {:noreply, init(:ok)}
  end

  @impl true
  def handle_info({:check_health, socket_path}, state) do
    # First, check if the Rust process port itself is alive
    if Process.alive?(state.port) do
      Logger.debug("Rust SIEM process port is alive.")
      # Then, check the health of the control socket
      case check_socket_health(socket_path) do
        :ok -> Logger.debug("Rust SIEM control socket is reachable.")
        {:error, _reason} -> Logger.warning("Rust SIEM control socket is NOT reachable.")
      end
    else
      Logger.error("Rust SIEM process port is dead. Attempting to restart.")
      # If port is dead, try to re-initialize the process
      # This will restart the port and update state
      {:noreply, init(:ok)}
    end
    # Reschedule the health check, passing the current socket_path from state
    Process.send_after(self(), {:check_health, state.socket_path}, @check_interval)
    {:noreply, state}
  end

  defp check_socket_health(socket_path) do
    # Attempt to connect to the UDS. If it succeeds, the control socket is likely up.
    # We don't need to send a command, just check connectivity.
    case :gen_tcp.connect({:local, ~c"#{socket_path}"}, 0, [:binary, packet: 0]) do
      {:ok, socket} ->
        :gen_tcp.close(socket) # Close immediately as we only checked connectivity
        :ok
      {:error, reason} ->
        # Log the error but don't necessarily crash. The process might be restarting or not ready.
        Logger.warning("Health check failed for socket #{socket_path}: #{inspect(reason)}")
        {:error, reason}
    end
  end
end


defmodule SiemSupervisor.ControlClient do

  @doc """
  Sends a command to a performer node's control socket.

  The `identifier` is expected to be the OS Process ID (PID) of the performer.
  """
  def send_command(os_pid, command) do
    # Construct the dynamic socket path using the OS PID
    socket_path = "/tmp/siem_control_performer_#{os_pid}.sock"
    Logger.debug("Attempting to send command to PID #{os_pid} via socket #{socket_path}")

    case :gen_tcp.connect({:local, ~c"#{socket_path}"}, 0, [:binary, packet: 0]) do
      {:ok, socket} ->
        # Ensure command ends with newline, as per original logic.
        :gen_tcp.send(socket, command <> "
")
        # In a real-world scenario, you might want to read the response here.
        # For now, we just send and close.
        :gen_tcp.close(socket)
        {:ok, socket} # Return socket for potential further interaction, or just :ok
      {:error, reason} ->
        Logger.warning("Failed to connect to control socket #{socket_path} for PID #{os_pid}: #{inspect(reason)}")
        {:error, reason}
    end
  end

  @doc """
  Updates the threshold for a specific performer node identified by its OS PID.
  """
  def update_threshold(os_pid, new_threshold) do
    send_command(os_pid, "SET_THRESHOLD #{new_threshold}")
  end

  @doc """
  Broadcasts a new threshold to all registered performer nodes.
  This function assumes that :siem_nodes ETS table contains OS PIDs as keys.
  If it contains other identifiers, this function would need adjustment
  to map those identifiers to OS PIDs.
  """
  def broadcast_threshold(new_threshold) do
    # Assuming :siem_nodes ETS table stores tuples like {os_pid, node_data}
    # :ets.lookup returns a list of {key, value} tuples.
    nodes_data = :ets.lookup(:siem_nodes)

    os_pids = Enum.map(nodes_data, fn {pid, _data} -> pid end)

    Logger.info("Broadcasting threshold #{new_threshold} to #{length(os_pids)} nodes.")
    for os_pid <- os_pids do
      update_threshold(os_pid, new_threshold)
    end
  end
end
