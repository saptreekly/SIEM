defmodule SiemSupervisor.RustProcess do
  use GenServer

  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: opts[:name])
  end

  def handle_info(:check_health, state) do
    # Check health of the remote control socket
    case SiemSupervisor.ControlClient.check_performer_health(state.ip_address, state.control_port) do
      :ok ->
        Logger.debug("Remote Rust Performer #{state.node_id} at #{state.ip_address}:#{state.control_port} is healthy.")
        {:noreply, %{state | status: :healthy}}
      {:error, reason} ->
        Logger.warning("Remote Rust Performer #{state.node_id} at #{state.ip_address}:#{state.control_port} is unreachable. Reason: #{inspect(reason)}.")
        {:noreply, %{state | status: :unreachable}}
    end
  end

  def handle_info({:EXIT, _pid, reason}, state) do
    Logger.error("GenServer for #{state.node_id} exited: #{inspect(reason)}")
    {:noreply, %{state | status: :exited}}
  end
end

defmodule SiemSupervisor.ControlClient do
  require Logger
  @default_control_port 8081

  @doc """
  Sends a command to a remote performer node's TCP control socket.

  The `ip_address` and `control_port` identify the target performer.
  """
  def send_command(ip_address, control_port \\ @default_control_port, command) do
    Logger.debug("Attempting to send command to #{ip_address}:#{control_port} via TCP.")

    case :gen_tcp.connect(to_charlist(ip_address), control_port, [:binary, packet: 0]) do
      {:ok, socket} ->
        :gen_tcp.send(socket, command <> "\n")
        :gen_tcp.close(socket)
        :ok
      {:error, reason} ->
        Logger.error("Failed to connect to #{ip_address}:#{control_port}. Reason: #{inspect(reason)}.")
        {:error, reason}
    end
  end

  def check_performer_health(ip_address, control_port \\ @default_control_port) do
    case :gen_tcp.connect(to_charlist(ip_address), control_port, [:binary, packet: 0, active: false]) do
      {:ok, socket} ->
        :gen_tcp.close(socket)
        :ok
      {:error, reason} ->
        {:error, reason}
    end
  end

  def update_threshold(ip_address, control_port \\ @default_control_port, new_threshold) do
    send_command(ip_address, control_port, "SET_THRESHOLD #{new_threshold}")
  end

  def broadcast_threshold(new_threshold) do
    # Assuming :siem_nodes ETS table stores data like %{name: node_id, ip_address: ip, control_port: port}
    nodes_data = :ets.tab2list(:siem_nodes)

    Logger.info("Broadcasting threshold #{new_threshold} to #{length(nodes_data)} nodes.")
    for {_id, node_map} <- nodes_data do
      ip_address = Map.fetch!(node_map, :ip_address)
      control_port = Map.get(node_map, :control_port, @default_control_port)
      update_threshold(ip_address, control_port, new_threshold)
    end
  end
end
