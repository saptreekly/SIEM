defmodule SiemSupervisor.RustProcess do
  use GenServer
  require Logger

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  @impl true
  def init(:ok) do
    # Start the Rust SIEM process
    port = Port.open({:spawn, "/Users/jackweekly/Desktop/SIEM/target/debug/siem"}, [:binary, :exit_status])
    {:ok, %{port: port}}
  end

  @impl true
  def handle_info({_port, {:data, data}}, state) do
    Logger.info("Rust SIEM output: #{data}")
    {:noreply, state}
  end

  @impl true
  def handle_info({_port, {:exit_status, status}}, state) do
    Logger.error("Rust SIEM crashed with status #{status}. Restarting...")
    # Restart the process
    port = Port.open({:spawn, "/Users/jackweekly/Desktop/SIEM/target/debug/siem"}, [:binary, :exit_status])
    {:noreply, %{state | port: port}}
  end
end

defmodule SiemSupervisor.ControlClient do

  def send_command(node_id, command) do
    # Assuming each node has a dedicated socket path based on ID
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
