defmodule SiemSupervisor.GossipListener do
  use GenServer
  require Logger

  @port 9000
  @table :siem_nodes

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  @impl true
  def init(:ok) do
    :ets.new(@table, [:set, :public, :named_table, read_concurrency: true])
    {:ok, socket} = :gen_udp.open(@port, [:binary, active: true])
    Logger.info("Elixir Conductor Gossip Listener started on port #{@port}")
    {:ok, %{socket: socket}}
  end

  @impl true
  def handle_info({:udp, _socket, _ip, _port, data}, state) do
    case String.split(data, ":") do
      ["PERFORMER_ALIVE", node_id] ->
        # Update node status in ETS with current timestamp
        :ets.insert(@table, {node_id, System.system_time(:second)})
        Logger.debug("Performer node seen: #{node_id}")
      _ ->
        :ignore
    end
    {:noreply, state}
  end
end
