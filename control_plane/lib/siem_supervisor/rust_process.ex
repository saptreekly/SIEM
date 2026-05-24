defmodule SiemSupervisor.RustProcess do
  use GenServer
  require Logger

  @check_interval 5000 # 5 seconds
  @default_control_port 8081

  # We'll use this module to track the state of a remote Rust performer.
  # It will no longer spawn a local process.
  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: opts[:name])
  end

  @impl true
  def init(opts) do
    Process.flag(:trap_exit, true)
    ip_address = Keyword.fetch!(opts, :ip_address)
    control_port = Keyword.get(opts, :control_port, @default_control_port)
    node_id = Keyword.fetch!(opts, :name)

    Logger.info("Initializing remote Rust Performer tracker for #{node_id} at #{ip_address}:#{control_port}")

    # Schedule periodic health check
    Process.send_after(self(), :check_health, @check_interval)

    {:ok, %{ip_address: ip_address, control_port: control_port, node_id: node_id, status: :unknown}}
  end

  @impl true
  def handle_info(:check_health, state) do
    # Check health of the remote control socket
    case SiemSupervisor.ControlClient.check_performer_health(state.ip_address, state.control_port) do
      :ok ->
        Logger.debug("Remote Rust Performer #{state.node_id} at #{state.ip_address}:#{state.control_port} is healthy.")
        {:noreply, %{state | status: :healthy}}
      {:error, reason} ->
        Logger.warning("Remote Rust Performer #{state.node_id} at #{state.ip_address}:#{state.control_port} is NOT reachable: #{inspect(reason)}")
        {:noreply, %{state | status: :unreachable}}
    end

    Process.send_after(self(), :check_health, @check_interval)
    {:noreply, state}
  end

  # No longer handling exit_status or data from a local port.
  # These would be handled by a network connection monitoring system if needed.
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
  def send_command(ip_address, control_port \ @default_control_port, command) do
    Logger.debug("Attempting to send command to #{ip_address}:#{control_port} via TCP.")

    case :gen_tcp.connect(to_charlist(ip_address), control_port, [:binary, packet: 0]) do
      {:ok, socket} ->
        :gen_tcp.send(socket, command <> "
")
        :gen_tcp.close(socket)
        :ok
      {:error, reason} ->
        Logger.warning("Failed to connect to control socket #{ip_address}:#{control_port}: #{inspect(reason)}")
        {:error, reason}
    end
  end

  @doc """
  Checks the health of a remote performer node by attempting a TCP connection.
  """
  def check_performer_health(ip_address, control_port \ @default_control_port) do
    case :gen_tcp.connect(to_charlist(ip_address), control_port, [:binary, packet: 0, active: false, exit_on_close: false, send_timeout: 1000]) do
      {:ok, socket} ->
        :gen_tcp.close(socket)
        :ok
      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Updates the threshold for a specific remote performer node.
  """
  def update_threshold(ip_address, control_port \\ @default_control_port, new_threshold) do
    send_command(ip_address, control_port, "SET_THRESHOLD #{new_threshold}")
  end

  @doc """
  Broadcasts a new threshold to all registered performer nodes.
  This function assumes that :siem_nodes ETS table contains maps like
  `%{name: node_id, ip_address: ip, control_port: port}`.
  """
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
