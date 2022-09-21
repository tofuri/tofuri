use crate::{
    constants::{BLOCK_TIME_MIN, MIN_STAKE, SYNC_BLOCKS},
    p2p::MyBehaviour,
    print,
    stake::Stake,
    types, util,
};
use libp2p::{gossipsub::IdentTopic, Swarm};
use log::error;
use std::{
    error::Error,
    time::{Duration, SystemTime},
};
pub async fn next() {
    let mut nanos = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let secs = nanos / 1_000_000_000;
    nanos -= secs * 1_000_000_000;
    nanos = 1_000_000_000 - nanos;
    tokio::time::sleep(Duration::from_nanos(nanos as u64)).await
}
pub fn handle(swarm: &mut Swarm<MyBehaviour>) -> Result<(), Box<dyn Error>> {
    let behaviour = swarm.behaviour_mut();
    *behaviour.blockchain.get_heartbeats_mut() += 1;
    behaviour
        .blockchain
        .get_synchronizer_mut()
        .heartbeat_handle();
    handle_block(behaviour)?;
    handle_sync(behaviour)?;
    let millis = lag();
    print::heartbeat_lag(behaviour.blockchain.get_heartbeats(), millis);
    behaviour.blockchain.set_lag(millis);
    Ok(())
}
fn handle_block(behaviour: &mut MyBehaviour) -> Result<(), Box<dyn Error>> {
    let mut forge = true;
    if !behaviour.blockchain.get_stakers().is_empty() {
        if &behaviour.blockchain.get_stakers()[0].0
            != behaviour.blockchain.get_keypair().public.as_bytes()
            || util::timestamp()
                < behaviour.blockchain.get_latest_block().timestamp
                    + BLOCK_TIME_MIN as types::Timestamp
        {
            forge = false;
        }
    } else {
        let mut stake = Stake::new(true, MIN_STAKE, 0);
        stake.sign(behaviour.blockchain.get_keypair());
        behaviour.blockchain.set_cold_start_stake(stake);
    }
    if forge {
        match behaviour.blockchain.forge_block() {
            Ok(block) => {
                if behaviour.gossipsub.all_peers().count() > 0 {
                    behaviour
                        .gossipsub
                        .publish(IdentTopic::new("block"), bincode::serialize(&block)?)?;
                }
            }
            Err(err) => error!("{}", err),
        };
    }
    behaviour.blockchain.append_handle();
    Ok(())
}
fn handle_sync(behaviour: &mut MyBehaviour) -> Result<(), Box<dyn Error>> {
    if behaviour.blockchain.get_hashes().is_empty() {
        return Ok(());
    }
    if behaviour.gossipsub.all_peers().count() == 0 {
        return Ok(());
    }
    for _ in 0..SYNC_BLOCKS {
        let block = behaviour.blockchain.get_next_sync_block();
        behaviour
            .gossipsub
            .publish(IdentTopic::new("block"), bincode::serialize(&block)?)?;
    }
    Ok(())
}
fn lag() -> f64 {
    let mut micros = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros();
    let secs = micros / 1_000_000;
    micros -= secs * 1_000_000;
    micros as f64 / 1_000_f64
}
