use clap::ArgMatches;
use slog::{debug, info, o, trace, warn};
use std::any::type_name;
use std::path::PathBuf;
use std::sync::Arc;

use eth2::ssz::{Decode, Encode};
use eth2::types::EnrForkId;
use eth2::utils::{create_topic_ids, get_fork_id_from_dir, get_fork_id_from_string};
use types::events::Events;

#[cfg(not(feature = "local"))]
use mothra::{Mothra, NetworkGlobals, NetworkMessage};
#[cfg(feature = "local")]
use mothra_local::{Mothra, NetworkGlobals, NetworkMessage};

// Holds variables needed to interacts with mothra
pub struct Adapter {
    network_globals: Arc<NetworkGlobals>,
    network_send: tokio_01::sync::mpsc::UnboundedSender<NetworkMessage>,
    network_exit: tokio_01::sync::oneshot::Sender<()>,
    enr_fork_id: Option<eth2::types::EnrForkId>,
    log: slog::Logger,
}

impl Adapter {
    pub fn new(
        executor: &tokio_compat::runtime::TaskExecutor,
        client_name: String,
        platform: String,
        protocol_version: String,
        testnet_dir: Option<PathBuf>,
        arg_matches: &ArgMatches<'_>,
        log: slog::Logger,
    ) -> Self {
        let mut mothra_log = log.clone();

        // get mothra subcommand args matches
        let mothra_arg_matches = &arg_matches.subcommand_matches("mothra").unwrap();

        // if debug-level is set in subcommand
        // Note: bc mothra sets default value to info
        // mothra_arg_matches.is_present is always true
        // so we have to use mothra_arg_matches.occurrences_of
        if mothra_arg_matches.occurrences_of("debug-level") > 0 {
            let debug_level = mothra_arg_matches.value_of("debug-level").unwrap();
            // re-configure logging
            mothra_log = utils::config_logger(debug_level, false).new(o!("P2PAdapter" => "Mothra"));
        }

        // NOTE:  The reason the bootnode must be parsed form the CLI instead of using the Enr type
        // from mothra directly is bc Enr is defined in both Mothra and LH (which is a problem)
        let boot_nodes: Vec<String> = if mothra_arg_matches.is_present("boot-nodes") {
            let boot_enr_str = mothra_arg_matches.value_of("boot-nodes").unwrap();
            boot_enr_str
                .split(',')
                .map(|x| x.into())
                .collect::<Vec<String>>()
        } else {
            Default::default()
        };

        let mut config = Mothra::get_config(
            Some(client_name),
            Some(platform),
            Some(protocol_version),
            &mothra_arg_matches,
        );

        // TODO
        // Option: Learn fork_id from supplied cli arg directly

        // Option: Learn fork_id from bootnode
        let (enr_fork_id, enr_fork_id_bytes) = match get_fork_id_from_string(boot_nodes[0].clone())
        {
            Some(enr_fork_id) => {
                // configure gossip topics
                config.network_config.topics = create_topic_ids(enr_fork_id.clone());
                (
                    Some(enr_fork_id.clone()),
                    enr_fork_id.clone().as_ssz_bytes(),
                )
            }
            _ => {
                // Option: Learn fork_id from supplied testnet_dir
                match get_fork_id_from_dir(testnet_dir) {
                    Some(enr_fork_id) => {
                        // configure gossip topics
                        config.network_config.topics = create_topic_ids(enr_fork_id.clone());
                        (
                            Some(enr_fork_id.clone()),
                            enr_fork_id.clone().as_ssz_bytes(),
                        )
                    }
                    _ => (None, [0u8, 32].to_vec()),
                }
            }
        };

        // instantiate mothra
        let (network_globals, network_send, network_exit) = Mothra::new(
            config,
            enr_fork_id_bytes,
            &executor,
            on_discovered_peer,
            on_receive_gossip,
            on_receive_rpc,
            mothra_log,
        )
        .unwrap();

        Adapter {
            network_globals,
            network_send,
            network_exit,
            enr_fork_id,
            log,
        }
    }

    pub fn close(self) -> Result<(), ()> {
        self.network_exit.send(())
    }
}

fn on_discovered_peer(peer: String) {
    println!("new peer discovered");
    println!("peer: {:?}", peer);
}

fn on_receive_gossip(topic: String, data: Vec<u8>) {
    println!("gossip message received");
    println!("topic: {:?}", topic);
    println!("data: {:?}", String::from_utf8_lossy(&data));
}

fn on_receive_rpc(method: String, req_resp: u8, peer: String, data: Vec<u8>) {
    println!("rpc message received");
    println!("method: {:?}", method);
    println!("req_resp: {:?}", req_resp);
    println!("peer: {:?}", peer);
    println!("data: {:?}", String::from_utf8_lossy(&data));
}