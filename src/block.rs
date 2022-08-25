use crate::{db, stake::Stake, transaction::Transaction, types, util};
use ed25519::signature::Signer;
use ed25519_dalek::{Keypair, PublicKey, Signature};
use rocksdb::{DBWithThreadMode, SingleThreaded};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::error::Error;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub previous_hash: types::Hash,
    pub timestamp: types::Timestamp,
    pub public_key: types::PublicKey,
    #[serde(with = "BigArray")]
    pub signature: types::Signature,
    pub transactions: Vec<Transaction>,
    pub stakes: Vec<Stake>,
}
impl Block {
    pub fn from(
        previous_hash: types::Hash,
        timestamp: types::Timestamp,
        public_key: types::PublicKey,
        signature: types::Signature,
    ) -> Block {
        Block {
            previous_hash,
            timestamp,
            public_key,
            signature,
            transactions: vec![],
            stakes: vec![],
        }
    }
    pub fn new(previous_hash: types::Hash) -> Block {
        Block::from(previous_hash, util::timestamp(), [0; 32], [0; 64])
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
        BlockMetadataLean::put(db, &block_metadata.hash(), block_metadata_lean)?;
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
        let mut block = Block::from(
            block_metadata_lean.previous_hash,
            block_metadata_lean.timestamp,
            block_metadata_lean.public_key,
            block_metadata_lean.signature,
        );
        for hash in block_metadata_lean.transaction_hashes {
            block.transactions.push(Transaction::get(db, &hash)?);
        }
        for hash in block_metadata_lean.stake_hashes {
            block.stakes.push(Stake::get(db, &hash)?);
        }
        Ok(block)
    }
    pub fn has_valid_transactions(&self) -> bool {
        for transaction in self.transactions.iter() {
            if !transaction.is_valid() {
                return false;
            }
        }
        true
    }
    pub fn has_valid_stakes(&self) -> bool {
        for stake in self.stakes.iter() {
            if !stake.is_valid() {
                return false;
            }
        }
        true
    }
    pub fn is_valid(&self) -> bool {
        self.has_valid_transactions()
            && self.has_valid_stakes()
            && self.timestamp <= util::timestamp()
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
    pub public_key: types::PublicKey,
    pub signature: types::Signature,
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
    pub fn hash(&self) -> types::Hash {
        util::hash(&bincode::serialize(&BlockHeader::from(self)).unwrap())
    }
    pub fn transaction_hashes(transactions: &Vec<Transaction>) -> Vec<types::Hash> {
        let mut transaction_hashes = vec![];
        for transaction in transactions {
            transaction_hashes.push(transaction.hash());
        }
        transaction_hashes
    }
    pub fn stake_hashes(stakes: &Vec<Stake>) -> Vec<types::Hash> {
        let mut stake_hashes = vec![];
        for stake in stakes {
            stake_hashes.push(stake.hash());
        }
        stake_hashes
    }
    pub fn merkle_root(transaction_hashes: &[types::Hash]) -> types::MerkleRoot {
        util::CBMT::build_merkle_root(transaction_hashes)
    }
    pub fn sign(&mut self, keypair: &Keypair) {
        self.public_key = keypair.public.to_bytes();
        self.signature = keypair.sign(&self.hash()).to_bytes();
    }
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        let public_key: PublicKey = PublicKey::from_bytes(&self.public_key)?;
        let signature: Signature = Signature::from_bytes(&self.signature)?;
        Ok(public_key.verify_strict(&self.hash(), &signature)?)
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct BlockMetadataLean {
    pub previous_hash: types::Hash,
    pub timestamp: types::Timestamp,
    pub public_key: types::PublicKey,
    #[serde(with = "BigArray")]
    pub signature: types::Signature,
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
            db::cf_handle_blocks(db)?,
            hash,
            bincode::serialize(&block_metadata_lean)?,
        )?;
        Ok(())
    }
    pub fn get(
        db: &DBWithThreadMode<SingleThreaded>,
        hash: &[u8],
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(db
            .get_cf(db::cf_handle_blocks(db)?, hash)?
            .ok_or("block not found")?)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[test]
    fn test_hash() {
        let block = Block::from([0; 32], 0, [0; 32], [0; 64]);
        let block_metadata = BlockMetadata::from(&block);
        assert_eq!(
            [
                0xad, 0x6a, 0xc9, 0x4e, 0x2d, 0xe8, 0xac, 0xda, 0xc7, 0x2c, 0x22, 0x8b, 0x4d, 0x0e,
                0x0c, 0xb6, 0xd9, 0x85, 0x66, 0x0e, 0xa0, 0x03, 0x0c, 0x0a, 0x9d, 0x0a, 0x6f, 0xf4,
                0xa9, 0x4e, 0xc5, 0xe4
            ],
            block_metadata.hash()
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
        let block_metadata_lean = BlockMetadataLean::from(&block_metadata);
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
        let block_metadata = BlockMetadata::from(&block);
        b.iter(|| block_metadata.hash());
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
