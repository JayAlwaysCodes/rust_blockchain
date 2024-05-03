use crate::error::Result;
use serde::{Deserialize, Serialize};
use crypto::{digest::Digest, sha2::Sha256};


///Transaction represents a Bitcoin transaction

#[derive(Serialize, Deserialize, Debug, Clone)]

pub struct Transaction{
    pub id: String,
    pub vin: Vec<TXInput>,
    pub vout: Vec<TXOutput>,
}

///TXInput represents transaction input

#[derive(Serialize, Deserialize, Debug, Clone)]

pub struct TXInput{
    pub txid: String,
    pub vout: i32,
    pub script_sig: String,
}

///TXOutput represents a transaction output

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXOutput{
    pub value: i32,
    pub script_pub_key: String,
}

impl Transaction{
    pub fn new_coinbase(to: String, mut data: String) -> Result<Transaction> {
        if data == String::from(""){
            data += &format!("Reward to '{}' ", to);
        }
        let mut tx = Transaction {
            id: String::new(),
            vin: vec![TXInput {
                txid: String::new(),
                vout: -1,
                script_sig: data,
            }],

            vout: vec![TXOutput{
                value: 100,
                script_pub_key: to,
            }],
        };
        tx.set_id()?;
        Ok(tx)
    }

    ///SetID sets ID of a transaction
    fn set_id(&mut self) -> Result<()>{
        let mut hasher  = Sha256::new();
        let data = bincode::serialize(self)?;
        hasher.input(&data);
        self.id = hasher.result_str();
        Ok(())
    }

    ///IsCoinbase checks whether the transaction is coinbase
    pub fn is_coinbase(&self) -> bool {
        self.vin.len() == 1 && self.vin[0].txid.is_empty() && self.vin[0].vout == -1
    }
    
}

impl TXInput {
    ///CanUnlockOutputWith checks whether the address initiated the transaction
    pub fn can_unlock_output_with(&self, unlocking_data: &str) -> bool {
        self.script_sig == unlocking_data
    }
}

impl TXOutput {
    ///CanBeUnlockedWith checks if the output can be unlocked with the provided data
    pub fn can_be_unlock_with(&self, unlocking_data: &str) -> bool{
        self.script_pub_key == unlocking_data
    }
}