defmodule SiemSupervisor.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      SiemSupervisor.GossipListener,
      SiemSupervisor.RustProcess
    ]

    opts = [strategy: :one_for_one, name: SiemSupervisor.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
