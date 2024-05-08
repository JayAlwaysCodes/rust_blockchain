use crate::error::Result;
use crate::block::{self, Block};
use crate::transaction:: Transaction;
use crate::tx::{self, TXOutput, TXOutputs};
use bincode;
use failure::format_err;
use std::string::String;
use log::info;
use std::collections::HashMap;
const TARGET_HEXT: usize = 4;

#[derive(Debug, Clone)]
pub struct Blockchain {
    current_hash: String,
    db: sled::Db,
}

pub struct BlockchainIter<'a> {
    current_hash: String,
    bc: &'a Blockchain,
}

impl Blockchain {
   ///NewBlockchain creates a new Blockchain db
    pub fn new() -> Result<Blockchain> {
        info!("open blockchain");

        let db = sled::open("data/blocks")?;
        let hash = db.get("LAST")?.expect("Must create a new block database first");
        info!("Found block database");
        let lasthash = String::from_utf8(hash.to_vec())?;
        Ok(Blockchain{
            current_hash: lasthash.clone(),
            db,
        }) 
    }
    ///CreateBlockchain creates a new blockchain DB
    pub fn create_blockchain(address: String) -> Result<Blockchain>{
        info!("Creating new blockchain...");
        if let Err(e) = std::fs::remove_dir_all("data/blocks"){
            info!("blocks not exist to delete")
        }

        let db = sled::open("data/blocks")?;
        info!("Creating new block database...");
        let cbtx = Transaction::new_coinbase(address, String::from("GENESIS_COINBASE_DATA"))?;
        let genesis: Block = Block::new_genesis_block(cbtx);
        db.insert(genesis.get_hash(), bincode::serialize(&genesis)?)?;
        db.insert("LAST", genesis.get_hash().as_bytes())?;
        let bc = Blockchain {
            current_hash: genesis.get_hash(),
            db,
        };
        bc.db.flush()?;
        Ok(bc)
    }
    pub fn add_block(&mut self, transaction: Vec<Transaction>) -> Result<Block> {
        let lasthash = self.db.get("LAST")?.unwrap();
        let new_block = Block::new_block(transaction, String::from_utf8(lasthash.to_vec())?, TARGET_HEXT)?;
        self.db
            .insert(new_block.get_hash(), bincode::serialize(&new_block)?)?;
        self.db.insert("LAST", new_block.get_hash().as_bytes())?;
        self.current_hash = new_block.get_hash();
        Ok(new_block)
    }

    ///FindUnspentTransactions returns a list of transactions containing unspent outputs
    fn  find_unspent_transactions(&self, address: &[u8]) -> Vec<Transaction> {
        let mut  spent_TXOs: HashMap<String, Vec<i32>> = HashMap::new();
        let mut unspend_TXs: Vec<Transaction> = Vec::new();

        for block in self.iter(){
            for tx in block.get_transaction(){
                for index in 0..tx.vout.len(){
                    if let Some(ids) = spent_TXOs.get(&tx.id) {
                        if ids.contains(&(index as i32)){
                            continue;
                        }
                    }

                    if tx.vout[index].can_be_unlock_with(address){
                        unspend_TXs.push(tx.to_owned())
                    }
                }

                if !tx.is_coinbase() {
                    for i in &tx.vin {
                        if i.can_unlock_output_with(address) {
                            match spent_TXOs.get_mut(&i.txid){
                                Some(v) => {
                                    v.push(i.vout);
                                }
                                None => {
                                    spent_TXOs.insert(i.txid.clone(), vec![i.vout]);
                                }
                            }
                        }
                    }
                }
            }
        }

        unspend_TXs
    }

    /// FindUTXO finds and returns all unspent transaction outputs
    pub fn find_UTXO(&self) -> HashMap<String, TXOutputs> {
        let mut utxos: HashMap<String, TXOutputs> = HashMap::new();
        let mut spend_txos: HashMap<String, Vec<i32>> = HashMap::new();


        for block in self.iter(){
            for tx in block.get_transaction(){
               for index in 0..tx.vout.len(){
                    if let Some(ids) = spend_txos.get(&tx.id){
                        if ids.contains(&(index as i32)){
                            continue;
                        }
                    }

                    match utxos.get_mut(&tx.id){
                        Some(v) => {
                            v.outputs.push(tx.vout[index].clone());
                        }
                        None => {
                            utxos.insert(tx.id.clone(), TXOutputs{outputs: vec![tx.vout[index].clone()],},);
                        }
                    }
                }

                if !tx.is_coinbase() {
                    for i in &tx.vin {
                        match spend_txos.get_mut(&i.txid){
                            Some(v) => {
                                v.push(i.vout);
                            }
                            None => {
                                spend_txos.insert(i.txid.clone(), vec![i.vout]);
                            }
                        }
                    }
                }
            }

        }
        

        utxos
    }
 
    pub fn iter(&self) -> BlockchainIter {
        BlockchainIter {
            current_hash: self.current_hash.clone(),
            bc: &self,
        }
    }

    ///FindTransaction finds a transaction by its ID
    pub fn find_transaction(&self, id: &str) -> Result<Transaction>{
        for b in self.iter(){
            for tx in b.get_transaction(){
                if tx.id == id{
                    return Ok(tx.clone());
                }
            }
        }
        Err(format_err!("Transaction is not found"))

    }

    fn get_prev_TXs(&self, tx: &Transaction) -> Result<HashMap<String, Transaction>>{
        let mut prev_TXs = HashMap::new();
        for vin in &tx.vin{
            let prev_TX = self.find_transaction(&vin.txid)?;
            prev_TXs.insert(prev_TX.id.clone(), prev_TX);
        }
        Ok(prev_TXs)
    }

    ///SignTransaction signs inputs of a Transactionn
    pub fn sign_transaction(&self, tx: &mut Transaction, private_key: &[u8]) -> Result<()>{
        let prev_TXs = self.get_prev_TXs(tx)?;
        tx.sign(private_key, prev_TXs);
        Ok(())
    }

    ///VerifyTransaction verifies transaction input signatures
    pub fn  verify_transaction(&self, tx: &mut Transaction) -> Result<bool> {
        let prev_TXs = self.get_prev_TXs(tx)?;
        tx.verify(prev_TXs)
    }


}

impl<'a> Iterator for BlockchainIter<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(encode_block) = self.bc.db.get(&self.current_hash) {
            return match encode_block {
                Some(b) => {
                    if let Ok(block) = bincode::deserialize::<Block>(&b) {
                        self.current_hash = block.get_prev_hash();
                        Some(block)
                    } else {
                        None
                    }
                }
                None => None,
            };
        }
        None
    }
}
