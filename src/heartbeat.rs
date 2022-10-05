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
    heartbeats(behaviour);
    sync(behaviour)?;
    if behaviour.heartbeats % TPS != 0 {
        return Ok(());
    }
    message_data_hashes(behaviour);
    block(behaviour)?;
    behaviour.blockchain.heartbeat_handle();
    lag(behaviour);
    Ok(())
}
fn heartbeats(behaviour: &mut MyBehaviour) {
    behaviour.heartbeats += 1;
}
fn message_data_hashes(behaviour: &mut MyBehaviour) {
    behaviour.message_data_hashes.clear();
}
fn block(behaviour: &mut MyBehaviour) -> Result<(), Box<dyn Error>> {
    let states = behaviour.blockchain.get_states();
    let mut forge = true;
    if *behaviour.blockchain.get_syncing() {
        forge = false;
    }
    if forge {
        let timestamp = util::timestamp();
        if let Some(public_key) = states
            .dynamic
            .get_staker(timestamp, states.dynamic.get_latest_block().timestamp)
        {
            if public_key != behaviour.blockchain.get_keypair().public.as_bytes()
                || timestamp
                    < states.dynamic.get_latest_block().timestamp
                        + BLOCK_TIME_MIN as types::Timestamp
            {
                forge = false;
            }
        } else {
            let mut stake = Stake::new(true, MIN_STAKE, 0);
            stake.sign(behaviour.blockchain.get_keypair());
            behaviour.blockchain.set_cold_start_stake(stake);
        }
    }
    if forge {
        let block = behaviour.blockchain.forge_block()?;
        let data = bincode::serialize(&block)?;
        if !behaviour.filter(&data, true) && behaviour.gossipsub.all_peers().count() > 0 {
            behaviour
                .gossipsub
                .publish(IdentTopic::new("block"), data)?;
        }
    }
    behaviour.blockchain.append_handle();
    Ok(())
}
fn sync(behaviour: &mut MyBehaviour) -> Result<(), Box<dyn Error>> {
    if behaviour
        .blockchain
        .get_states()
        .dynamic
        .get_hashes()
        .is_empty()
    {
        return Ok(());
    }
    if behaviour.gossipsub.all_peers().count() == 0 {
        *behaviour.blockchain.get_sync_index_mut() = 0;
        return Ok(());
    }
    for _ in 0..SYNC_BLOCKS_PER_TICK {
        let block = behaviour.blockchain.get_next_sync_block();
        let data = bincode::serialize(&block)?;
        behaviour
            .gossipsub
            .publish(IdentTopic::new("block"), data)?;
    }
    Ok(())
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
