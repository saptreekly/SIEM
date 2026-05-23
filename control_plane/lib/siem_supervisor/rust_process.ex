defmodule SiemSupervisor.RustProcess do
  use GenServer
  require Logger

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, :ok, name: __MODULE__)
  end

  @impl true
  def init(:ok) do
    # Start the Rust SIEM process
    port = Port.open({:spawn, "./target/release/siem"}, [:binary, :exit_status])
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
    port = Port.open({:spawn, "./target/release/siem"}, [:binary, :exit_status])
    {:noreply, %{state | port: port}}
  end
end

defmodule SiemSupervisor.ControlClient do
  def send_command(command) do
    # Connect to the UDS created by Rust
    case :gen_tcp.connect({:local, '/tmp/siem_control.sock'}, 0, [:binary, packet: 0]) do
      {:ok, socket} ->
        :gen_tcp.send(socket, command <> "
")
        {:ok, socket}
      {:error, reason} ->
        {:error, reason}
    end
  end
end
