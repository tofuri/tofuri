use crate::Node;
use crate::CARGO_PKG_NAME;
use crate::CARGO_PKG_REPOSITORY;
use crate::CARGO_PKG_VERSION;
use crate::GIT_HASH;
use address::public;
use api::BlockHex;
use api::Root;
use api::StakeHex;
use api::TransactionHex;
use axum::extract::Path;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::routing::post;
use axum::Json;
use axum::Router;
use axum::Server;
use block::Block;
use chrono::offset::Utc;
use fork::BLOCK_TIME;
use hex;
use serde::de::DeserializeOwned;
use stake::Stake;
use std::convert::TryInto;
use std::net::IpAddr;
use std::net::SocketAddr;
use sync::Sync;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::error;
use transaction::Transaction;
pub fn spawn(client: Client, addr: &SocketAddr) {
    async fn root() -> impl IntoResponse {
        Json(Root {
            cargo_pkg_name: CARGO_PKG_NAME.to_string(),
            cargo_pkg_version: CARGO_PKG_VERSION.to_string(),
            cargo_pkg_repository: CARGO_PKG_REPOSITORY.to_string(),
            git_hash: GIT_HASH.to_string(),
        })
    }
    async fn cargo_pkg_name() -> impl IntoResponse {
        Json(CARGO_PKG_NAME)
    }
    async fn cargo_pkg_version() -> impl IntoResponse {
        Json(CARGO_PKG_VERSION)
    }
    async fn cargo_pkg_repository() -> impl IntoResponse {
        Json(CARGO_PKG_REPOSITORY)
    }
    async fn git_hash() -> impl IntoResponse {
        Json(GIT_HASH)
    }
    async fn balance(State(client): State<Client>, address: Path<String>) -> impl IntoResponse {
        let address_bytes = public::decode(&address).unwrap();
        Json(client.call::<u128>(Call::Balance(address_bytes)).await)
    }
    async fn balance_pending_min(
        State(client): State<Client>,
        address: Path<String>,
    ) -> impl IntoResponse {
        let address_bytes = public::decode(&address).unwrap();
        Json(
            client
                .call::<u128>(Call::BalancePendingMin(address_bytes))
                .await,
        )
    }
    async fn balance_pending_max(
        State(client): State<Client>,
        address: Path<String>,
    ) -> impl IntoResponse {
        let address_bytes = public::decode(&address).unwrap();
        Json(
            client
                .call::<u128>(Call::BalancePendingMax(address_bytes))
                .await,
        )
    }
    async fn staked(State(client): State<Client>, address: Path<String>) -> impl IntoResponse {
        let address_bytes = public::decode(&address).unwrap();
        Json(client.call::<u128>(Call::Staked(address_bytes)).await)
    }
    async fn staked_pending_min(
        State(client): State<Client>,
        address: Path<String>,
    ) -> impl IntoResponse {
        let address_bytes = public::decode(&address).unwrap();
        Json(
            client
                .call::<u128>(Call::StakedPendingMin(address_bytes))
                .await,
        )
    }
    async fn staked_pending_max(
        State(client): State<Client>,
        address: Path<String>,
    ) -> impl IntoResponse {
        let address_bytes = public::decode(&address).unwrap();
        Json(
            client
                .call::<u128>(Call::StakedPendingMax(address_bytes))
                .await,
        )
    }
    async fn height(State(client): State<Client>) -> impl IntoResponse {
        Json(client.call::<usize>(Call::Height).await)
    }
    async fn height_by_hash(State(client): State<Client>, hash: Path<String>) -> impl IntoResponse {
        let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
        Json(client.call::<usize>(Call::HeightByHash(hash)).await)
    }
    async fn block_latest(State(client): State<Client>) -> impl IntoResponse {
        let block = client.call::<Block>(Call::BlockLatest).await;
        let block_hex: BlockHex = block.try_into().unwrap();
        Json(block_hex)
    }
    async fn hash_by_height(
        State(client): State<Client>,
        height: Path<String>,
    ) -> impl IntoResponse {
        let height: usize = height.parse().unwrap();
        let hash = client.call::<[u8; 32]>(Call::HashByHeight(height)).await;
        let hash_hex = hex::encode(hash);
        Json(hash_hex)
    }
    async fn block_by_hash(State(client): State<Client>, hash: Path<String>) -> impl IntoResponse {
        let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
        let block = client.call::<Block>(Call::BlockByHash(hash)).await;
        let block_hex: BlockHex = block.try_into().unwrap();
        Json(block_hex)
    }
    async fn transaction_by_hash(
        State(client): State<Client>,
        hash: Path<String>,
    ) -> impl IntoResponse {
        let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
        let transaction = client
            .call::<Transaction>(Call::TransactionByHash(hash))
            .await;
        let transaction_hex: TransactionHex = transaction.try_into().unwrap();
        Json(transaction_hex)
    }
    async fn stake_by_hash(State(client): State<Client>, hash: Path<String>) -> impl IntoResponse {
        let hash: [u8; 32] = hex::decode(hash.clone()).unwrap().try_into().unwrap();
        let stake = client.call::<Stake>(Call::StakeByHash(hash)).await;
        let stake_hex: StakeHex = stake.try_into().unwrap();
        Json(stake_hex)
    }
    async fn peers(State(client): State<Client>) -> impl IntoResponse {
        Json(client.call::<Vec<IpAddr>>(Call::Peers).await)
    }
    async fn peer(State(client): State<Client>, Path(ip_addr): Path<String>) -> impl IntoResponse {
        let ip_addr = ip_addr.parse().unwrap();
        Json(client.call::<bool>(Call::Peer(ip_addr)).await)
    }
    async fn transaction(
        State(client): State<Client>,
        Json(transaction): Json<TransactionHex>,
    ) -> impl IntoResponse {
        let transaction: Transaction = transaction.try_into().unwrap();
        Json(client.call::<bool>(Call::Transaction(transaction)).await)
    }
    async fn stake(State(client): State<Client>, Json(stake): Json<StakeHex>) -> impl IntoResponse {
        let stake: Stake = stake.try_into().unwrap();
        Json(client.call::<bool>(Call::Stake(stake)).await)
    }
    async fn address(State(client): State<Client>) -> impl IntoResponse {
        Json(public::encode(
            &client.call::<[u8; 20]>(Call::Address).await,
        ))
    }
    async fn ticks(State(client): State<Client>) -> impl IntoResponse {
        Json(client.call::<usize>(Call::Ticks).await)
    }
    async fn time() -> impl IntoResponse {
        Json(chrono::offset::Utc::now().timestamp_millis())
    }
    async fn tree_size(State(client): State<Client>) -> impl IntoResponse {
        Json(client.call::<usize>(Call::TreeSize).await)
    }
    async fn sync(State(client): State<Client>) -> impl IntoResponse {
        let sync = client.call::<Sync>(Call::Sync).await;
        Json(sync)
    }
    async fn random_queue(State(client): State<Client>) -> impl IntoResponse {
        Json(
            client
                .call::<Vec<[u8; 20]>>(Call::RandomQueue)
                .await
                .iter()
                .map(public::encode)
                .collect::<Vec<_>>(),
        )
    }
    async fn unstable_hashes(State(client): State<Client>) -> impl IntoResponse {
        Json(client.call::<usize>(Call::UnstableHashes).await)
    }
    async fn unstable_latest_hashes(State(client): State<Client>) -> impl IntoResponse {
        Json(
            client
                .call::<Vec<[u8; 32]>>(Call::UnstableLatestHashes)
                .await
                .iter()
                .map(hex::encode)
                .collect::<Vec<_>>(),
        )
    }
    async fn unstable_stakers(State(client): State<Client>) -> impl IntoResponse {
        Json(client.call::<usize>(Call::UnstableStakers).await)
    }
    async fn stable_hashes(State(client): State<Client>) -> impl IntoResponse {
        Json(client.call::<usize>(Call::StableHashes).await)
    }
    async fn stable_latest_hashes(State(client): State<Client>) -> impl IntoResponse {
        Json(
            client
                .call::<Vec<[u8; 32]>>(Call::StableLatestHashes)
                .await
                .iter()
                .map(hex::encode)
                .collect::<Vec<_>>(),
        )
    }
    async fn stable_stakers(State(client): State<Client>) -> impl IntoResponse {
        Json(client.call::<usize>(Call::StableStakers).await)
    }
    async fn sync_remaining(State(client): State<Client>) -> impl IntoResponse {
        let sync = client.call::<Sync>(Call::Sync).await;
        if sync.completed {
            return Json(0.0);
        }
        if !sync.downloading() {
            return Json(-1.0);
        }
        let block = client.call::<Block>(Call::BlockLatest).await;
        let mut diff = (Utc::now().timestamp() as u32).saturating_sub(block.timestamp) as f32;
        diff /= BLOCK_TIME as f32;
        diff /= sync.bps;
        Json(diff)
    }
    let builder = Server::bind(addr);
    let router = Router::new()
        .route("/", get(root))
        .route("/balance/:address", get(balance))
        .route("/balance_pending_min/:address", get(balance_pending_min))
        .route("/balance_pending_max/:address", get(balance_pending_max))
        .route("/staked/:address", get(staked))
        .route("/staked_pending_min/:address", get(staked_pending_min))
        .route("/staked_pending_max/:address", get(staked_pending_max))
        .route("/height", get(height))
        .route("/height/:hash", get(height_by_hash))
        .route("/block", get(block_latest))
        .route("/hash/:height", get(hash_by_height))
        .route("/block/:hash", get(block_by_hash))
        .route("/transaction/:hash", get(transaction_by_hash))
        .route("/stake/:hash", get(stake_by_hash))
        .route("/peers", get(peers))
        .route("/peer/:ip_addr", get(peer))
        .route("/transaction", post(transaction))
        .route("/stake", post(stake))
        .route("/cargo_pkg_name", get(cargo_pkg_name))
        .route("/cargo_pkg_version", get(cargo_pkg_version))
        .route("/cargo_pkg_repository", get(cargo_pkg_repository))
        .route("/git_hash", get(git_hash))
        .route("/address", get(address))
        .route("/ticks", get(ticks))
        .route("/time", get(time))
        .route("/tree_size", get(tree_size))
        .route("/sync", get(sync))
        .route("/random_queue", get(random_queue))
        .route("/unstable_hashes", get(unstable_hashes))
        .route("/unstable_latest_hashes", get(unstable_latest_hashes))
        .route("/unstable_stakers", get(unstable_stakers))
        .route("/stable_hashes", get(stable_hashes))
        .route("/stable_latest_hashes", get(stable_latest_hashes))
        .route("/stable_stakers", get(stable_stakers))
        .route("/sync_remaining", get(sync_remaining))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(client);
    let make_service = router.into_make_service();
    tokio::spawn(async { builder.serve(make_service).await });
}
pub enum Call {
    Balance([u8; 20]),
    BalancePendingMin([u8; 20]),
    BalancePendingMax([u8; 20]),
    Staked([u8; 20]),
    StakedPendingMin([u8; 20]),
    StakedPendingMax([u8; 20]),
    Height,
    HeightByHash([u8; 32]),
    BlockLatest,
    HashByHeight(usize),
    BlockByHash([u8; 32]),
    TransactionByHash([u8; 32]),
    StakeByHash([u8; 32]),
    Peers,
    Peer(IpAddr),
    Transaction(Transaction),
    Stake(Stake),
    Address,
    Ticks,
    TreeSize,
    Sync,
    RandomQueue,
    UnstableHashes,
    UnstableLatestHashes,
    UnstableStakers,
    StableHashes,
    StableLatestHashes,
    StableStakers,
}
pub fn channel(buffer: usize) -> (Client, mpsc::Receiver<Request>) {
    let (tx, rx) = mpsc::channel(buffer);
    (Client(tx), rx)
}
#[derive(Clone)]
pub struct Client(pub mpsc::Sender<Request>);
impl Client {
    pub async fn call<T: DeserializeOwned>(&self, call: Call) -> T {
        let (tx, rx) = oneshot::channel();
        let _ = self.0.send(Request { call, tx }).await;
        let response = rx.await.unwrap();
        bincode::deserialize(&response.0).unwrap()
    }
}
pub struct Request {
    pub call: Call,
    pub tx: oneshot::Sender<Response>,
}
pub struct Response(pub Vec<u8>);
#[derive(Debug)]
pub enum Error {
    Blockchain(blockchain::Error),
    DB(db::Error),
    Bincode(bincode::Error),
}
pub async fn accept(node: &mut Node, request: Request) {
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
        bincode::serialize(&db::block::get(&node.db, &hash).map_err(Error::DB)?)
            .map_err(Error::Bincode)
    }
    fn transaction_by_hash(node: &mut Node, hash: [u8; 32]) -> Result<Vec<u8>, Error> {
        bincode::serialize(&db::transaction::get(&node.db, &hash).map_err(Error::DB)?)
            .map_err(Error::Bincode)
    }
    fn stake_by_hash(node: &mut Node, hash: [u8; 32]) -> Result<Vec<u8>, Error> {
        bincode::serialize(&db::stake::get(&node.db, &hash).map_err(Error::DB)?)
            .map_err(Error::Bincode)
    }
    fn peers(node: &mut Node) -> Result<Vec<u8>, Error> {
        bincode::serialize(&node.p2p.connections.values().collect::<Vec<_>>())
            .map_err(Error::Bincode)
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
    let res = match request.call {
        Call::Balance(a) => balance(node, a),
        Call::BalancePendingMin(a) => balance_pending_min(node, a),
        Call::BalancePendingMax(a) => balance_pending_max(node, a),
        Call::Staked(a) => staked(node, a),
        Call::StakedPendingMin(a) => staked_pending_min(node, a),
        Call::StakedPendingMax(a) => staked_pending_max(node, a),
        Call::Height => height(node),
        Call::HeightByHash(a) => height_by_hash(node, a),
        Call::BlockLatest => block_latest(node),
        Call::HashByHeight(a) => hash_by_height(node, a),
        Call::BlockByHash(a) => block_by_hash(node, a),
        Call::TransactionByHash(a) => transaction_by_hash(node, a),
        Call::StakeByHash(a) => stake_by_hash(node, a),
        Call::Peers => peers(node),
        Call::Peer(a) => peer(node, a),
        Call::Transaction(a) => transaction(node, a),
        Call::Stake(a) => stake(node, a),
        Call::Address => address(node),
        Call::Ticks => ticks(node),
        Call::TreeSize => tree_size(node),
        Call::Sync => sync(node),
        Call::RandomQueue => random_queue(node),
        Call::UnstableHashes => unstable_hashes(node),
        Call::UnstableLatestHashes => unstable_latest_hashes(node),
        Call::UnstableStakers => unstable_stakers(node),
        Call::StableHashes => stable_hashes(node),
        Call::StableLatestHashes => stable_latest_hashes(node),
        Call::StableStakers => stable_stakers(node),
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
