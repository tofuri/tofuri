use crate::{
    blockchain::Blockchain, constants::MIN_STAKE, db, stake::Stake, stakers,
    transaction::Transaction, types, util,
};
use ed25519::signature::Signer;
use rocksdb::{DBWithThreadMode, SingleThreaded};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::{error::Error, fmt};
#[derive(Serialize, Deserialize, Clone)]
pub struct Block {
    pub previous_hash: types::Hash,
    pub timestamp: types::Timestamp,
    pub public_key: types::PublicKeyBytes,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
    pub transactions: Vec<Transaction>,
    pub stakes: Vec<Stake>,
}
impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #![allow(dead_code)]
        #[derive(Debug)]
        struct Block {
            hash: String,
            previous_hash: String,
            timestamp: u32,
            public_key: String,
            signature: String,
            transactions: Vec<String>,
            stakes: Vec<String>,
        }
        write!(
            f,
            "{:?}",
            Block {
                hash: hex::encode(self.hash()),
                previous_hash: hex::encode(self.previous_hash),
                timestamp: self.timestamp,
                public_key: hex::encode(self.public_key),
                signature: hex::encode(self.signature),
                transactions: self
                    .transactions
                    .iter()
                    .map(|x| hex::encode(x.hash()))
                    .collect(),
                stakes: self.stakes.iter().map(|x| hex::encode(x.hash())).collect(),
            }
        )
    }
}
impl Block {
    pub fn from(
        previous_hash: types::Hash,
        timestamp: types::Timestamp,
        public_key: types::PublicKeyBytes,
        signature: types::SignatureBytes,
        transactions: Vec<Transaction>,
        stakes: Vec<Stake>,
    ) -> Block {
        Block {
            previous_hash,
            timestamp,
            public_key,
            signature,
            transactions,
            stakes,
        }
    }
    pub fn new(previous_hash: types::Hash) -> Block {
        Block::from(
            previous_hash,
            util::timestamp(),
            [0; 32],
            [0; 64],
            vec![],
            vec![],
        )
    }
    pub fn sign(&mut self, keypair: &types::Keypair) {
        self.public_key = keypair.public.to_bytes();
        self.signature = keypair.sign(&self.hash()).to_bytes();
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        let public_key = types::PublicKey::from_bytes(&self.public_key)?;
        let signature = types::Signature::from_bytes(&self.signature)?;
        Ok(public_key.verify_strict(&self.hash(), &signature)?)
    }
    pub fn hash(&self) -> types::Hash {
        let block_metadata = BlockMetadata::from(self);
        util::hash(&bincode::serialize(&BlockHeader::from(&block_metadata)).unwrap())
    }
    pub fn put(&self, db: &DBWithThreadMode<SingleThreaded>) -> Result<(), Box<dyn Error>> {
        let block_metadata = BlockMetadata::from(self);
        let block_metadata_lean = BlockMetadataLean::from(&block_metadata);
        for transaction in self.transactions.iter() {
            transaction.put(db)?;
        }
        for stake in self.stakes.iter() {
            stake.put(db)?;
        }
        BlockMetadataLean::put(db, &self.hash(), block_metadata_lean)?;
        Ok(())
    }
    pub fn get(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &[u8],
    ) -> Result<Block, Box<dyn Error>> {
        Block::load(db, &BlockMetadataLean::get(db, hash)?)
    }
    pub fn load(
        db: &DBWithThreadMode<SingleThreaded>,
        bytes: &[u8],
    ) -> Result<Block, Box<dyn Error>> {
        let block_metadata_lean: BlockMetadataLean = bincode::deserialize(bytes)?;
        let mut transactions = vec![];
        for hash in block_metadata_lean.transaction_hashes {
            transactions.push(Transaction::get(db, &hash)?);
        }
        let mut stakes = vec![];
        for hash in block_metadata_lean.stake_hashes {
            stakes.push(Stake::get(db, &hash)?);
        }
        Ok(Block::from(
            block_metadata_lean.previous_hash,
            block_metadata_lean.timestamp,
            block_metadata_lean.public_key,
            block_metadata_lean.signature,
            transactions,
            stakes,
        ))
    }
    pub fn fees(&self) -> types::Amount {
        let mut fees = 0;
        for transaction in self.transactions.iter() {
            fees += transaction.fee;
        }
        for stake in self.stakes.iter() {
            fees += stake.fee;
        }
        fees
    }
    pub fn reward(&self, balance_staked: types::Amount) -> types::Amount {
        self.fees() + util::reward(balance_staked)
    }
    pub fn validate(&self, blockchain: &Blockchain) -> Result<(), Box<dyn Error>> {
        let db = blockchain.get_db();
        // let height = if self.previous_hash == [0; 32] {
        // 0
        // } else {
        // blockchain.get_tree().height(&self.previous_hash)?
        // };
        // if height + TRUST_FORK_AFTER_BLOCKS < blockchain.get_height() {
        // return Err("block doesn't extend untrusted fork".into());
        // }
        if self.previous_hash != [0; 32] && blockchain.get_tree().get(&self.previous_hash).is_none()
        {
            return Err("block doesn't extend chain".into());
        }
        let mut balance_public_keys = vec![];
        let mut balance_staked_public_keys = vec![];
        for transaction in self.transactions.iter() {
            balance_public_keys.push(transaction.public_key_input);
        }
        for stake in self.stakes.iter() {
            balance_staked_public_keys.push(stake.public_key);
        }
        let (balances, balances_staked) = blockchain.get_balances_at_hash(
            db,
            balance_public_keys,
            balance_staked_public_keys,
            self.previous_hash,
        );
        if self.previous_hash != [0; 32] {
            let stakers = stakers::get(blockchain.get_db(), &self.previous_hash)?;
            if let Some((public_key, _)) = stakers.get(0) {
                if public_key != &self.public_key {
                    return Err("block isn't signed by the staker first in queue".into());
                }
            }
        }
        let public_key_inputs = self
            .transactions
            .iter()
            .map(|t| t.public_key_input)
            .collect::<Vec<types::PublicKeyBytes>>();
        if (1..public_key_inputs.len())
            .any(|i| public_key_inputs[i..].contains(&public_key_inputs[i - 1]))
        {
            return Err("block includes multiple transactions from same public_key_input".into());
        }
        let public_keys = self
            .stakes
            .iter()
            .map(|s| s.public_key)
            .collect::<Vec<types::PublicKeyBytes>>();
        if (1..public_keys.len()).any(|i| public_keys[i..].contains(&public_keys[i - 1])) {
            return Err("block includes multiple stakes from same public_key".into());
        }
        if self.verify().is_err() {
            return Err("block has invalid signature".into());
        }
        if self.timestamp > util::timestamp() {
            return Err("block has invalid timestamp (block is from the future)".into());
        }
        if Block::get(db, &self.hash()).is_ok() {
            return Err("block already in db".into());
        }
        if !self.stakes.is_empty() {
            let stake = self.stakes.get(0).unwrap();
            if stake.fee == 0 {
                if self.stakes.len() != 1 {
                    return Err("only allowed to mint 1 stake".into());
                }
                if stake.verify().is_err() {
                    return Err("mint stake has invalid signature".into());
                }
                if stake.timestamp > util::timestamp() {
                    return Err(
                        "mint stake has invalid timestamp (mint stake is from the future)".into(),
                    );
                }
                if stake.timestamp < self.timestamp {
                    return Err("mint stake too old".into());
                }
                if !stake.deposit {
                    return Err("mint stake must be deposit".into());
                }
                if stake.amount != MIN_STAKE {
                    return Err("mint stake invalid amount".into());
                }
                if stake.fee != 0 {
                    return Err("mint stake invalid fee".into());
                }
            } else {
                for stake in self.stakes.iter() {
                    let balance = balances.get(&stake.public_key).unwrap();
                    let balance_staked = balances_staked.get(&stake.public_key).unwrap();
                    stake.validate(db, *balance, *balance_staked, self.timestamp)?;
                }
            }
        }
        for transaction in self.transactions.iter() {
            let balance = balances.get(&transaction.public_key_input).unwrap();
            transaction.validate(db, *balance, self.timestamp)?;
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct BlockHeader {
    pub previous_hash: types::Hash,
    pub transaction_merkle_root: types::MerkleRoot,
    pub stake_merkle_root: types::MerkleRoot,
    pub timestamp: types::Timestamp,
}
impl BlockHeader {
    pub fn from(block: &BlockMetadata) -> BlockHeader {
        BlockHeader {
            previous_hash: block.previous_hash,
            transaction_merkle_root: block.transaction_merkle_root,
            stake_merkle_root: block.stake_merkle_root,
            timestamp: block.timestamp,
        }
    }
}
#[derive(Debug)]
pub struct BlockMetadata {
    pub previous_hash: types::Hash,
    pub timestamp: types::Timestamp,
    pub public_key: types::PublicKeyBytes,
    pub signature: types::SignatureBytes,
    pub transaction_hashes: Vec<types::Hash>,
    pub transaction_merkle_root: types::MerkleRoot,
    pub stake_hashes: Vec<types::Hash>,
    pub stake_merkle_root: types::MerkleRoot,
}
impl BlockMetadata {
    pub fn from(block: &Block) -> BlockMetadata {
        let transaction_hashes = BlockMetadata::transaction_hashes(&block.transactions);
        let stake_hashes = BlockMetadata::stake_hashes(&block.stakes);
        BlockMetadata {
            previous_hash: block.previous_hash,
            timestamp: block.timestamp,
            public_key: block.public_key,
            signature: block.signature,
            transaction_merkle_root: BlockMetadata::merkle_root(&transaction_hashes),
            transaction_hashes,
            stake_merkle_root: BlockMetadata::merkle_root(&stake_hashes),
            stake_hashes,
        }
    }
    fn transaction_hashes(transactions: &Vec<Transaction>) -> Vec<types::Hash> {
        let mut transaction_hashes = vec![];
        for transaction in transactions {
            transaction_hashes.push(transaction.hash());
        }
        transaction_hashes
    }
    fn stake_hashes(stakes: &Vec<Stake>) -> Vec<types::Hash> {
        let mut stake_hashes = vec![];
        for stake in stakes {
            stake_hashes.push(stake.hash());
        }
        stake_hashes
    }
    fn merkle_root(hashes: &[types::Hash]) -> types::MerkleRoot {
        types::CBMT::build_merkle_root(hashes)
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct BlockMetadataLean {
    pub previous_hash: types::Hash,
    pub timestamp: types::Timestamp,
    pub public_key: types::PublicKeyBytes,
    #[serde(with = "BigArray")]
    pub signature: types::SignatureBytes,
    pub transaction_hashes: Vec<types::Hash>,
    pub stake_hashes: Vec<types::Hash>,
}
impl BlockMetadataLean {
    pub fn from(block_metadata: &BlockMetadata) -> BlockMetadataLean {
        BlockMetadataLean {
            previous_hash: block_metadata.previous_hash,
            timestamp: block_metadata.timestamp,
            public_key: block_metadata.public_key,
            signature: block_metadata.signature,
            transaction_hashes: block_metadata.transaction_hashes.to_vec(),
            stake_hashes: block_metadata.stake_hashes.to_vec(),
        }
    }
    pub fn put(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &types::Hash,
        block_metadata_lean: BlockMetadataLean,
    ) -> Result<(), Box<dyn Error>> {
        db.put_cf(
            db::blocks(db),
            hash,
            bincode::serialize(&block_metadata_lean)?,
        )?;
        Ok(())
    }
    pub fn get(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &[u8],
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(db.get_cf(db::blocks(db), hash)?.ok_or("block not found")?)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[test]
    fn test_hash() {
        let block = Block::from([0; 32], 0, [0; 32], [0; 64], vec![], vec![]);
        println!("{:x?}", block.hash());
        assert_eq!(
            [
                0xac, 0x6f, 0x86, 0xff, 0xf6, 0x30, 0xa5, 0x6a, 0x21, 0xf5, 0x9d, 0x3a, 0x0c, 0x1c,
                0x69, 0x07, 0xfe, 0x3f, 0x7c, 0xaf, 0xd5, 0xfa, 0x91, 0x6f, 0x9b, 0x72, 0x20, 0x32,
                0xf6, 0x05, 0x9e, 0xd9
            ],
            block.hash()
        );
    }
    #[bench]
    fn bench_metadata_from(b: &mut Bencher) {
        let block = Block::new([0; 32]);
        b.iter(|| BlockMetadata::from(&block));
    }
    #[bench]
    fn bench_header_from_metadata(b: &mut Bencher) {
        let block = Block::new([0; 32]);
        let block_metadata = BlockMetadata::from(&block);
        b.iter(|| BlockHeader::from(&block_metadata));
    }
    #[bench]
    fn bench_metadata_lean_from_metadata(b: &mut Bencher) {
        let block = Block::new([0; 32]);
        let block_metadata = BlockMetadata::from(&block);
        b.iter(|| BlockMetadataLean::from(&block_metadata));
    }
    #[bench]
    fn bench_bincode_serialize_header(b: &mut Bencher) {
        let block = Block::new([0; 32]);
        let block_metadata = BlockMetadata::from(&block);
        let block_header = BlockHeader::from(&block_metadata);
        println!("{:?}", block_header);
        println!("{:?}", bincode::serialize(&block_header));
        println!("{:?}", bincode::serialize(&block_header).unwrap().len());
        b.iter(|| bincode::serialize(&block_header));
    }
    #[bench]
    fn bench_bincode_serialize(b: &mut Bencher) {
        let block = Block::new([0; 32]);
        let block_metadata = BlockMetadata::from(&block);
        let mut block_metadata_lean = BlockMetadataLean::from(&block_metadata);
        block_metadata_lean.signature = [0xff; 64];
        block_metadata_lean.timestamp = util::timestamp();
        println!("{:?}", block_metadata_lean);
        println!("{:?}", bincode::serialize(&block_metadata_lean));
        println!(
            "{:?}",
            bincode::serialize(&block_metadata_lean).unwrap().len()
        );
        b.iter(|| bincode::serialize(&block_metadata_lean));
    }
    #[bench]
    fn bench_bincode_deserialize(b: &mut Bencher) {
        let block = Block::new([0; 32]);
        let block_metadata = BlockMetadata::from(&block);
        let block_metadata_lean = BlockMetadataLean::from(&block_metadata);
        let bytes = bincode::serialize(&block_metadata_lean).unwrap();
        b.iter(|| {
            let _: BlockMetadataLean = bincode::deserialize(&bytes).unwrap();
        });
    }
    #[bench]
    fn bench_hash(b: &mut Bencher) {
        let block = Block::new([0; 32]);
        b.iter(|| block.hash());
    }
    #[bench]
    fn bench_merkle_root_1(b: &mut Bencher) {
        let mut block = Block::new([0; 32]);
        for i in 0..1 {
            block.transactions.push(Transaction::new([0; 32], i, i));
        }
        let transaction_hashes = BlockMetadata::transaction_hashes(&block.transactions);
        b.iter(|| BlockMetadata::merkle_root(&transaction_hashes));
    }
    #[bench]
    fn bench_merkle_root_10(b: &mut Bencher) {
        let mut block = Block::new([0; 32]);
        for i in 0..10 {
            block.transactions.push(Transaction::new([0; 32], i, i));
        }
        let transaction_hashes = BlockMetadata::transaction_hashes(&block.transactions);
        b.iter(|| BlockMetadata::merkle_root(&transaction_hashes));
    }
    #[bench]
    fn bench_merkle_root_100(b: &mut Bencher) {
        let mut block = Block::new([0; 32]);
        for i in 0..100 {
            block.transactions.push(Transaction::new([0; 32], i, i));
        }
        let transaction_hashes = BlockMetadata::transaction_hashes(&block.transactions);
        b.iter(|| BlockMetadata::merkle_root(&transaction_hashes));
    }
    #[bench]
    fn bench_merkle_root_1000(b: &mut Bencher) {
        let mut block = Block::new([0; 32]);
        for i in 0..1000 {
            block.transactions.push(Transaction::new([0; 32], i, i));
        }
        let transaction_hashes = BlockMetadata::transaction_hashes(&block.transactions);
        b.iter(|| BlockMetadata::merkle_root(&transaction_hashes));
    }
}
