use crate::{
    bisq::{constants::BaseCurrencyNetwork, payload::*},
    p2p::{dispatch::*, Connection, ConnectionId, Request},
    prelude::*,
};
use std::{process, time::SystemTime};

#[derive(Debug, Clone, Copy)]
struct DummyDispatcher;
impl Dispatcher for DummyDispatcher {
    fn dispatch(&self, conn: ConnectionId, msg: network_envelope::Message) -> Dispatch {
        Dispatch::Consumed
    }
}

pub fn check_node(network: BaseCurrencyNetwork, addr: NodeAddress, proxy_port: u16) {
    let sys = System::new("risq");
    Arbiter::spawn(
        Connection::open(
            addr.clone(),
            network.into(),
            DummyDispatcher,
            Some(proxy_port),
        )
        .map_err(|_| {
            eprintln!("Could not open a connection.");
            process::exit(1);
        })
        .and_then(move |(_id, conn)| {
            let send_time = SystemTime::now();
            let ping = Ping {
                nonce: gen_nonce(),
                last_round_trip_time: 0,
            };
            println!("Sending Ping to {:?}", addr);
            conn.send(Request(ping))
                .map_err(|_| {
                    eprintln!("There was an issue sending Ping");
                    process::exit(2)
                })
                .map(move |pong| match pong {
                    Ok(_) => {
                        let ret = SystemTime::now();
                        println!(
                            "Received Pong after {}ms",
                            ret.duration_since(send_time)
                                .expect("Pong before Ping")
                                .as_millis()
                        );
                        process::exit(0)
                    }
                    Err(_) => {
                        eprintln!("There was an issue while awaiting the Pong response");
                        process::exit(3)
                    }
                })
        }),
    );;
    let _ = sys.run();
}