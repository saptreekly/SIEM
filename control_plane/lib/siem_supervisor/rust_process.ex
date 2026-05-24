defmodule SiemSupervisor.RustProcess do
  use GenServer
  require Logger

  @rust_binary "./target/release/siem"

  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: opts[:name] || __MODULE__)
  end

  def init(opts) do
    Process.flag(:trap_exit, true)
    
    # Launch the Rust core as a managed Port process
    port = Port.open({:spawn, @rust_binary}, [:exit_status, :binary, :stderr_to_stdout])
    
    {:ok, %{port: port, node_id: opts[:node_id], status: :healthy}}
  end

  def handle_info({port, {:exit_status, status}}, %{port: port} = state) do
    Logger.error("Rust core exited with status #{status}. Waiting 2 seconds before restarting...")
    # Give the OS time to release ports
    Process.sleep(2000)
    
    # Restart the port
    new_port = Port.open({:spawn, @rust_binary}, [:exit_status, :binary, :stderr_to_stdout])
    {:noreply, %{state | port: new_port, status: :restarting}}
  end

  def handle_info({port, {:data, data}}, %{port: port} = state) do
    Logger.debug("Rust core output: #{data}")
    {:noreply, state}
  end

  def handle_info(:check_health, state) do
    # Still keep health check for internal status management if needed
    {:noreply, state}
  end

  def handle_info(msg, state) do
    Logger.debug("Unexpected message: #{inspect(msg)}")
    {:noreply, state}
  end

  def terminate(reason, state) do
    Logger.info("Rust process manager terminating. Reason: #{inspect(reason)}")
    Port.close(state.port)
    :ok
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
