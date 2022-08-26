use crate::{
    constants::{BLOCKS_PER_SECOND_THRESHOLD, BLOCK_TIME_MIN, MAX_STAKE},
    p2p::MyBehaviour,
    print,
    stake::Stake,
    sync::Sync,
    types, util,
};
use colored::*;
use libp2p::{gossipsub::IdentTopic, Swarm};
use log::{debug, error, info};
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
    behaviour.validator.heartbeats += 1;
    behaviour.validator.synchronizer.heartbeat_handle();
    handle_block(behaviour)?;
    handle_sync(behaviour)?;
    let millis = lag();
    print::heartbeat_lag(behaviour.validator.heartbeats, millis);
    behaviour.validator.lag.rotate_right(1);
    behaviour.validator.lag[0] = millis;
    Ok(())
}
fn handle_block(behaviour: &mut MyBehaviour) -> Result<(), Box<dyn Error>> {
    if behaviour.validator.synchronizer.bps >= BLOCKS_PER_SECOND_THRESHOLD {
        return Ok(());
    }
    let mut forge = true;
    if !behaviour.validator.blockchain.stakers.is_empty() {
        if &behaviour.validator.blockchain.stakers[0].0
            != behaviour.validator.keypair.public.as_bytes()
            || util::timestamp()
                < behaviour.validator.blockchain.latest_block.timestamp
                    + BLOCK_TIME_MIN as types::Timestamp
        {
            forge = false;
        }
    } else {
        // cold start
        let mut stake = Stake::new(true, MAX_STAKE, 0);
        stake.sign(&behaviour.validator.keypair);
        behaviour.validator.blockchain.pending_stakes.push(stake);
    }
    if forge {
        // forge new block
        match behaviour
            .validator
            .blockchain
            .forge_block(&behaviour.validator.db, &behaviour.validator.keypair)
        {
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
    // accept forged blocks
    if let Err(err) = behaviour
        .validator
        .blockchain
        .accept_block(&behaviour.validator.db, forge)
    {
        debug!("{}", err)
    }
    Ok(())
}
fn handle_sync(behaviour: &mut MyBehaviour) -> Result<(), Box<dyn Error>> {
    if behaviour.validator.synchronizer.bps < BLOCKS_PER_SECOND_THRESHOLD {
        return Ok(());
    }
    info!(
        "{}: {} @ {}bps",
        "Synchronize".cyan(),
        behaviour
            .validator
            .blockchain
            .latest_height()
            .to_string()
            .yellow(),
        behaviour.validator.synchronizer.bps.to_string().yellow()
    );
    if behaviour.gossipsub.all_peers().count() > 0 {
        behaviour.gossipsub.publish(
            IdentTopic::new("sync"),
            bincode::serialize(&Sync::new(
                behaviour.validator.blockchain.latest_height() + 1,
            ))?,
        )?;
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
