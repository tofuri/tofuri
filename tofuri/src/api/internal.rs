use crate::api::Get;
use crate::Node;
use serde::de::DeserializeOwned;
use std::net::IpAddr;
use tofuri_stake::Stake;
use tofuri_transaction::Transaction;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::error;
#[derive(Debug)]
pub enum Error {
    Blockchain(tofuri_blockchain::Error),
    DB(tofuri_db::Error),
    Bincode(bincode::Error),
}
pub struct Request {
    pub get: Get,
    pub tx: oneshot::Sender<Response>,
}
pub struct Response(pub Vec<u8>);
#[derive(Clone)]
pub struct Internal(pub mpsc::Sender<Request>);
impl Internal {
    pub async fn get<T: DeserializeOwned>(&self, get: Get) -> T {
        let (tx, rx) = oneshot::channel();
        let _ = self.0.send(Request { get, tx }).await;
        let response = rx.await.unwrap();
        bincode::deserialize(&response.0).unwrap()
    }
}
pub async fn accept(node: &mut Node, request: Request) {
    let res = match request.get {
        Get::Balance(a) => balance(node, a),
        Get::BalancePendingMin(a) => balance_pending_min(node, a),
        Get::BalancePendingMax(a) => balance_pending_max(node, a),
        Get::Staked(a) => staked(node, a),
        Get::StakedPendingMin(a) => staked_pending_min(node, a),
        Get::StakedPendingMax(a) => staked_pending_max(node, a),
        Get::Height => height(node),
        Get::HeightByHash(a) => height_by_hash(node, a),
        Get::BlockLatest => block_latest(node),
        Get::HashByHeight(a) => hash_by_height(node, a),
        Get::BlockByHash(a) => block_by_hash(node, a),
        Get::TransactionByHash(a) => transaction_by_hash(node, a),
        Get::StakeByHash(a) => stake_by_hash(node, a),
        Get::Peers => peers(node),
        Get::Peer(a) => peer(node, a),
        Get::Transaction(a) => transaction(node, a),
        Get::Stake(a) => stake(node, a),
        Get::Address => address(node),
        Get::Ticks => ticks(node),
        Get::TreeSize => tree_size(node),
        Get::Sync => sync(node),
        Get::RandomQueue => random_queue(node),
        Get::UnstableHashes => unstable_hashes(node),
        Get::UnstableLatestHashes => unstable_latest_hashes(node),
        Get::UnstableStakers => unstable_stakers(node),
        Get::StableHashes => stable_hashes(node),
        Get::StableLatestHashes => stable_latest_hashes(node),
        Get::StableStakers => stable_stakers(node),
    };
    match res {
        Ok(vec) => {
            let _ = request.tx.send(Response(vec));
        }
        Err(e) => {
            error!(?e);
        }
    };
}
fn balance(node: &mut Node, address: [u8; 20]) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.balance(&address)).map_err(Error::Bincode)
}
fn balance_pending_min(node: &mut Node, address: [u8; 20]) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.balance_pending_min(&address)).map_err(Error::Bincode)
}
fn balance_pending_max(node: &mut Node, address: [u8; 20]) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.balance_pending_max(&address)).map_err(Error::Bincode)
}
fn staked(node: &mut Node, address: [u8; 20]) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.staked(&address)).map_err(Error::Bincode)
}
fn staked_pending_min(node: &mut Node, address: [u8; 20]) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.staked_pending_min(&address)).map_err(Error::Bincode)
}
fn staked_pending_max(node: &mut Node, address: [u8; 20]) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.staked_pending_max(&address)).map_err(Error::Bincode)
}
fn height(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.height()).map_err(Error::Bincode)
}
fn height_by_hash(node: &mut Node, hash: [u8; 32]) -> Result<Vec<u8>, Error> {
    bincode::serialize(
        &node
            .blockchain
            .height_by_hash(&hash)
            .map_err(Error::Blockchain)?,
    )
    .map_err(Error::Bincode)
}
fn block_latest(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.forks.unstable.latest_block).map_err(Error::Bincode)
}
fn hash_by_height(node: &mut Node, height: usize) -> Result<Vec<u8>, Error> {
    bincode::serialize(
        &node
            .blockchain
            .hash_by_height(height)
            .map_err(Error::Blockchain)?,
    )
    .map_err(Error::Bincode)
}
fn block_by_hash(node: &mut Node, hash: [u8; 32]) -> Result<Vec<u8>, Error> {
    bincode::serialize(&tofuri_db::block::get(&node.db, &hash).map_err(Error::DB)?)
        .map_err(Error::Bincode)
}
fn transaction_by_hash(node: &mut Node, hash: [u8; 32]) -> Result<Vec<u8>, Error> {
    bincode::serialize(&tofuri_db::transaction::get(&node.db, &hash).map_err(Error::DB)?)
        .map_err(Error::Bincode)
}
fn stake_by_hash(node: &mut Node, hash: [u8; 32]) -> Result<Vec<u8>, Error> {
    bincode::serialize(&tofuri_db::stake::get(&node.db, &hash).map_err(Error::DB)?)
        .map_err(Error::Bincode)
}
fn peers(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.p2p.connections.values().collect::<Vec<_>>()).map_err(Error::Bincode)
}
fn peer(node: &mut Node, ip_addr: IpAddr) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.p2p.connections_unknown.insert(ip_addr)).map_err(Error::Bincode)
}
fn transaction(node: &mut Node, transaction: Transaction) -> Result<Vec<u8>, Error> {
    bincode::serialize(&{
        let vec = bincode::serialize(&transaction).map_err(Error::Bincode)?;
        match node
            .blockchain
            .pending_transactions_push(transaction, node.args.time_delta)
        {
            Ok(()) => {
                if let Err(e) = node.p2p.gossipsub_publish("transaction", vec) {
                    error!(?e);
                }
                "success".to_string()
            }
            Err(e) => {
                error!(?e);
                format!("{:?}", e)
            }
        }
    })
    .map_err(Error::Bincode)
}
fn stake(node: &mut Node, stake: Stake) -> Result<Vec<u8>, Error> {
    bincode::serialize(&{
        let vec = bincode::serialize(&stake).map_err(Error::Bincode)?;
        match node
            .blockchain
            .pending_stakes_push(stake, node.args.time_delta)
        {
            Ok(()) => {
                if let Err(e) = node.p2p.gossipsub_publish("stake", vec) {
                    error!(?e);
                }
                "success".to_string()
            }
            Err(e) => {
                error!(?e);
                format!("{:?}", e)
            }
        }
    })
    .map_err(Error::Bincode)
}
fn address(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.key.as_ref().map(|x| x.address_bytes())).map_err(Error::Bincode)
}
fn ticks(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.ticks).map_err(Error::Bincode)
}
fn tree_size(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.tree.size()).map_err(Error::Bincode)
}
fn sync(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.sync).map_err(Error::Bincode)
}
fn random_queue(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.forks.unstable.stakers_n(8)).map_err(Error::Bincode)
}
fn unstable_hashes(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.forks.unstable.hashes.len()).map_err(Error::Bincode)
}
fn unstable_latest_hashes(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(
        &node
            .blockchain
            .forks
            .unstable
            .hashes
            .iter()
            .rev()
            .take(16)
            .collect::<Vec<_>>(),
    )
    .map_err(Error::Bincode)
}
fn unstable_stakers(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.forks.unstable.stakers.len()).map_err(Error::Bincode)
}
fn stable_hashes(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.forks.stable.hashes.len()).map_err(Error::Bincode)
}
fn stable_latest_hashes(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(
        &node
            .blockchain
            .forks
            .stable
            .hashes
            .iter()
            .rev()
            .take(16)
            .collect::<Vec<_>>(),
    )
    .map_err(Error::Bincode)
}
fn stable_stakers(node: &mut Node) -> Result<Vec<u8>, Error> {
    bincode::serialize(&node.blockchain.forks.stable.stakers.len()).map_err(Error::Bincode)
}
