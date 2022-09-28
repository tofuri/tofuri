use crate::{
    constants::{BLOCK_TIME_MIN, MICROS, MIN_STAKE, NANOS, SYNC_BLOCKS_PER_TICK, TPS},
    p2p::{self, MyBehaviour},
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
    let secs = nanos / NANOS;
    nanos -= secs * NANOS;
    nanos = NANOS - nanos;
    tokio::time::sleep(Duration::from_nanos(nanos as u64)).await
}
pub fn handle(swarm: &mut Swarm<MyBehaviour>) -> Result<(), Box<dyn Error>> {
    let behaviour = swarm.behaviour_mut();
    *behaviour.blockchain.get_heartbeats_mut() += 1;
    handle_sync(behaviour)?;
    if behaviour.blockchain.get_heartbeats() % TPS != 0 {
        return Ok(());
    }
    behaviour.hashes.clear();
    handle_block(behaviour)?;
    let millis = lag();
    print::heartbeat_lag(behaviour.blockchain.get_heartbeats(), millis);
    behaviour.blockchain.set_lag(millis);
    Ok(())
}
fn handle_block(behaviour: &mut MyBehaviour) -> Result<(), Box<dyn Error>> {
    let mut forge = true;
    if !behaviour
        .blockchain
        .get_states()
        .get_current()
        .get_stakers()
        .is_empty()
    {
        if &behaviour
            .blockchain
            .get_states()
            .get_current()
            .get_stakers()[0]
            .0
            != behaviour.blockchain.get_keypair().public.as_bytes()
            || util::timestamp()
                < behaviour
                    .blockchain
                    .get_states()
                    .get_current()
                    .get_latest_block()
                    .timestamp
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
                let data = bincode::serialize(&block)?;
                if !p2p::filter(behaviour, &data) && behaviour.gossipsub.all_peers().count() > 0 {
                    behaviour
                        .gossipsub
                        .publish(IdentTopic::new("block"), data)?;
                }
            }
            Err(err) => error!("{}", err),
        };
    }
    behaviour.blockchain.append_handle();
    Ok(())
}
fn handle_sync(behaviour: &mut MyBehaviour) -> Result<(), Box<dyn Error>> {
    if behaviour
        .blockchain
        .get_states()
        .get_current()
        .get_hashes()
        .is_empty()
    {
        return Ok(());
    }
    if behaviour.gossipsub.all_peers().count() == 0 {
        return Ok(());
    }
    for _ in 0..SYNC_BLOCKS_PER_TICK {
        let block = behaviour.blockchain.get_next_sync_block();
        let data = bincode::serialize(&block)?;
        if p2p::filter(behaviour, &data) {
            continue;
        }
        behaviour
            .gossipsub
            .publish(IdentTopic::new("block"), data)?;
    }
    Ok(())
}
fn lag() -> f64 {
    let mut micros = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros();
    let secs = micros / MICROS;
    micros -= secs * MICROS;
    micros as f64 / 1_000_f64
}
