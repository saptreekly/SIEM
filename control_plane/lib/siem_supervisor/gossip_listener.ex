defmodule SiemSupervisor.GossipListener do
  use GenServer
  require Logger

  @port 9000
  @table :siem_nodes
  @prune_interval :timer.seconds(15)
  @node_timeout :timer.seconds(30)

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  @impl true
  def init(:ok) do
    :ets.new(@table, [:set, :public, :named_table, read_concurrency: true])
    {:ok, socket} = :gen_udp.open(@port, [:binary, active: true])
    :timer.send_interval(@prune_interval, :prune_nodes)
    Logger.info("Elixir Conductor Gossip Listener started on port #{@port}")
    {:ok, %{socket: socket}}
  end

  @impl true
  def handle_info(:prune_nodes, state) do
    now = System.system_time(:second)
    :ets.tab2list(@table)
    |> Enum.each(fn {node_id, last_seen} ->
      if now - last_seen > div(@node_timeout, 1000) do
        :ets.delete(@table, node_id)
        Logger.info("Pruning stale Performer node: #{node_id}")
      end
    end)
    {:noreply, state}
  end

  @impl true
  def handle_info({:udp, _socket, _ip, _port, data}, state) do
    case String.split(data, ":") do
      ["PERFORMER_ALIVE", node_id] ->
        :ets.insert(@table, {node_id, System.system_time(:second)})
        Logger.debug("Performer node seen: #{node_id}")
      _ ->
        :ignore
    end
    {:noreply, state}
  end
end
