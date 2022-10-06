use crate::{
    constants::{BLOCK_TIME_MIN, MICROS, MIN_STAKE, NANOS, SYNC_BLOCKS_PER_TICK, TPS},
    p2p::MyBehaviour,
    print,
    stake::Stake,
    types, util,
};
use libp2p::{gossipsub::IdentTopic, Swarm};
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
pub fn handler(swarm: &mut Swarm<MyBehaviour>) -> Result<(), Box<dyn Error>> {
    let behaviour = swarm.behaviour_mut();
    behaviour.heartbeats += 1;
    sync(behaviour)?;
    if behaviour.heartbeats % TPS != 0 {
        return Ok(());
    }
    message_data_hashes(behaviour);
    syncing(behaviour);
    block_forge(behaviour);
    pending_blocks_accept(behaviour);
    lag(behaviour);
    Ok(())
}
fn message_data_hashes(behaviour: &mut MyBehaviour) {
    behaviour.message_data_hashes.clear();
}
fn block_forge(behaviour: &mut MyBehaviour) {
    let states = &behaviour.blockchain.states;
    if behaviour.blockchain.sync.syncing {
        return;
    }
    let timestamp = util::timestamp();
    if let Some(public_key) = states
        .dynamic
        .staker(timestamp, states.dynamic.latest_block.timestamp)
    {
        if public_key != behaviour.blockchain.keypair.public.as_bytes()
            || timestamp
                < states.dynamic.latest_block.timestamp + BLOCK_TIME_MIN as types::Timestamp
        {
            return;
        }
    } else {
        let mut stake = Stake::new(true, MIN_STAKE, 0);
        stake.sign(&behaviour.blockchain.keypair);
        behaviour.blockchain.set_cold_start_stake(stake);
    }
    let block = behaviour.blockchain.forge_block().unwrap();
    let data = bincode::serialize(&block).unwrap();
    if !behaviour.filter(&data, true) && behaviour.gossipsub.all_peers().count() > 0 {
        behaviour
            .gossipsub
            .publish(IdentTopic::new("block"), data)
            .unwrap();
    }
}
fn pending_blocks_accept(behaviour: &mut MyBehaviour) {
    behaviour.blockchain.pending_blocks_accept();
}
fn sync(behaviour: &mut MyBehaviour) -> Result<(), Box<dyn Error>> {
    if behaviour.blockchain.states.dynamic.hashes.is_empty() {
        return Ok(());
    }
    if behaviour.gossipsub.all_peers().count() == 0 {
        behaviour.blockchain.sync.index = 0;
        return Ok(());
    }
    for _ in 0..SYNC_BLOCKS_PER_TICK {
        let block = behaviour.blockchain.sync_block();
        let data = bincode::serialize(&block)?;
        behaviour
            .gossipsub
            .publish(IdentTopic::new("block"), data)?;
    }
    Ok(())
}
fn syncing(behaviour: &mut MyBehaviour) {
    behaviour.blockchain.sync.handler();
}
fn lag(behaviour: &mut MyBehaviour) {
    let mut micros = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros();
    let secs = micros / MICROS;
    micros -= secs * MICROS;
    let millis = micros as f64 / 1_000_f64;
    behaviour.lag = millis;
    print::heartbeat_lag(&behaviour.heartbeats, millis);
}
