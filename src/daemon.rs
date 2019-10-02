use crate::bisq::constants::BaseCurrencyNetwork;
use crate::bootstrap::Bootstrap;
use crate::data_router::*;
use crate::dispatch::ActorDispatcher;
use crate::peers::Peers;
use crate::server::{self, TorConfig};
use actix::{Arbiter, System};

pub struct DaemonConfig {
    pub server_port: u16,
    pub network: BaseCurrencyNetwork,
    pub tor_config: Option<TorConfig>,
    pub tor_proxy_port: Option<u16>,
}
pub fn run(
    DaemonConfig {
        server_port,
        network,
        tor_config,
        tor_proxy_port,
    }: DaemonConfig,
) {
    let sys = System::new("risq");
    let data_router = DataRouter::start();
    let dispatcher = ActorDispatcher::<DataRouter, DataRouterDispatch>::new(data_router);
    let peers = Peers::start(network, dispatcher.clone());
    let bootstrap = Bootstrap::start(network, peers.clone(), dispatcher, tor_proxy_port);

    Arbiter::new().exec_fn(move || {
        server::start(server_port, peers, bootstrap, tor_config);
    });
    let _ = sys.run();
}
