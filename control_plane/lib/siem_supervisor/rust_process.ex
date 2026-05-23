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
    port = start_port(executable)
    # Schedule periodic health check
    Process.send_after(self(), :check_health, @check_interval)
    {:ok, %{port: port, executable: executable}}
  end

  defp start_port(executable) do
    # Ensure no stale socket
    # Note: socket file name is random in SIEM, would need to cleanup all
    # Or rely on SIEM to clean it up.
    Logger.info("Starting Rust SIEM: #{executable}")
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
    # Restart the process
    port = start_port(state.executable)
    {:noreply, %{state | port: port}}
  end

  @impl true
  def handle_info(:check_health, state) do
    # Simple check: if port is alive
    if Process.alive?(state.port) do
      Logger.debug("Rust SIEM is healthy.")
    else
      Logger.error("Rust SIEM port is dead. Restarting...")
      port = start_port(state.executable)
      state = %{state | port: port}
    end
    Process.send_after(self(), :check_health, @check_interval)
    {:noreply, state}
  end
end


defmodule SiemSupervisor.ControlClient do

  def send_command(node_id, command) do
    # Assuming each node has a dedicated socket path based on ID
    # Matches /tmp/siem_control_performer_*.sock
    socket_path = "/tmp/siem_control_#{node_id}.sock"
    case :gen_tcp.connect({:local, ~c"#{socket_path}"}, 0, [:binary, packet: 0]) do
      {:ok, socket} ->
        :gen_tcp.send(socket, command <> "\n")
        {:ok, socket}
      {:error, reason} ->
        {:error, reason}
    end
  end

  def update_threshold(node_id, new_threshold) do
    send_command(node_id, "SET_THRESHOLD #{new_threshold}")
  end

  def broadcast_threshold(new_threshold) do
    nodes = :ets.tab2list(:siem_nodes)
    for {node_id, _} <- nodes do
      update_threshold(node_id, new_threshold)
    end
  end
end
