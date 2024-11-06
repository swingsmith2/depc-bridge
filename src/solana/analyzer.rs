use std::str::FromStr;

use serde::Deserialize;
use solana_sdk::{pubkey::Pubkey, signature::Signature, system_program};
use solana_transaction_status::{
    parse_instruction::ParsedInstruction, EncodedConfirmedTransactionWithStatusMeta,
    EncodedTransaction, UiInstruction, UiMessage, UiParsedInstruction, UiTransactionStatusMeta,
};

#[derive(Debug)]
pub enum Error {
    NoMetaCanBeFoundFromTransaction,
    CannotParseInstructionValue,
    CannotParseNumber,
    UnknownProgramId,
    CannotParsePubkey,
    LamportsIsRequiredFromInfoValue,
    AmountIsRequiredFromInfoValue,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoMetaCanBeFoundFromTransaction => {
                write!(f, "no meta can be found from transaction")
            }
            Error::CannotParseInstructionValue => write!(f, "cannot parse instruction value"),
            Error::CannotParseNumber => write!(f, "cannot parse number"),
            Error::UnknownProgramId => write!(f, "unknown program-id"),
            Error::CannotParsePubkey => write!(f, "cannot parse pubkey"),
            Error::LamportsIsRequiredFromInfoValue => {
                write!(f, "lamports cannot be found from info value")
            }
            Error::AmountIsRequiredFromInfoValue => {
                write!(f, "lamports cannot be found from info value")
            }
        }
    }
}

#[derive(Deserialize)]
struct InstructionInfoValue {
    source: String,
    destination: String,
    lamports: Option<String>,
    amount: Option<String>,
}

#[derive(Deserialize)]
struct InstructionValue {
    info: InstructionInfoValue,
    r#type: String,
}

pub struct InstructionDetail {
    pub source: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
}

pub enum Instruction {
    SplToken(InstructionDetail),
    Solana(InstructionDetail),
}

pub struct Transaction {
    pub signature: Signature,
    pub fee: u64,
    pub timestamp: i64,
    pub instructions: Vec<Instruction>,
}

pub struct TransactionAnalyzer<'a> {
    transaction_meta: &'a EncodedConfirmedTransactionWithStatusMeta,
}

impl<'a> TransactionAnalyzer<'a> {
    pub fn new(transaction_meta: &'a EncodedConfirmedTransactionWithStatusMeta) -> Self {
        Self { transaction_meta }
    }

    pub fn parse(&self, signature: Signature, timestamp: i64) -> Result<Transaction, Error> {
        let mut transaction = Transaction {
            signature,
            fee: self.get_fee()?,
            timestamp,
            instructions: vec![],
        };
        for ix in self.strip_instructions()?.iter() {
            transaction.instructions.push(parse_instruction(ix)?);
        }
        Ok(transaction)
    }

    fn get_fee(&self) -> Result<u64, Error> {
        Ok(self.get_meta()?.fee)
    }

    fn get_meta(&self) -> Result<&UiTransactionStatusMeta, Error> {
        if let Some(meta) = self.transaction_meta.transaction.meta.as_ref() {
            Ok(meta)
        } else {
            Err(Error::NoMetaCanBeFoundFromTransaction)
        }
    }

    fn strip_instructions(&self) -> Result<Vec<&ParsedInstruction>, Error> {
        let mut instructions = vec![];
        let transaction = &self.transaction_meta.transaction.transaction;
        if let EncodedTransaction::Json(transaction) = transaction {
            if let UiMessage::Parsed(message) = &transaction.message {
                for instruction in message.instructions.iter() {
                    if let UiInstruction::Parsed(UiParsedInstruction::Parsed(instruction)) =
                        instruction
                    {
                        instructions.push(instruction);
                    }
                }
            }
        }
        Ok(instructions)
    }
}

fn parse_pubkey(s: &str) -> Result<Pubkey, Error> {
    if let Ok(pubkey) = Pubkey::from_str(s) {
        Ok(pubkey)
    } else {
        Err(Error::CannotParsePubkey)
    }
}

fn parse_number(s: &str) -> Result<u64, Error> {
    if let Ok(n) = s.parse() {
        Ok(n)
    } else {
        Err(Error::CannotParseNumber)
    }
}

fn parse_instruction(instruction: &ParsedInstruction) -> Result<Instruction, Error> {
    // parse instruction value
    let res = serde_json::from_value(instruction.parsed.clone());
    if res.is_err() {
        return Err(Error::CannotParseInstructionValue);
    }
    let instruction_value: InstructionValue = res.unwrap();
    // check and create the result
    let mut instruction_detail = InstructionDetail {
        source: parse_pubkey(&instruction_value.info.source)?,
        destination: parse_pubkey(&instruction_value.info.destination)?,
        amount: 0,
    };
    let program_id = parse_pubkey(&instruction.program_id)?;
    if program_id == system_program::id() {
        if let Some(amount) = instruction_value.info.lamports {
            instruction_detail.amount = parse_number(&amount)?;
            Ok(Instruction::Solana(instruction_detail))
        } else {
            Err(Error::LamportsIsRequiredFromInfoValue)
        }
    } else if program_id == spl_token::id() {
        if let Some(amount) = instruction_value.info.amount {
            instruction_detail.amount = parse_number(&amount)?;
            Ok(Instruction::SplToken(instruction_detail))
        } else {
            Err(Error::AmountIsRequiredFromInfoValue)
        }
    } else {
        Err(Error::UnknownProgramId)
    }
}

#[cfg(test)]
mod tests {
    use solana_client::rpc_client::RpcClient;
    use solana_sdk::commitment_config::CommitmentConfig;
    use solana_transaction_status::UiTransactionEncoding;

    use super::*;

    const DEFAULT_LOCAL_ENDPOINT: &str = "https://api.devnet.solana.com";
    const TEST_SIGNATURE_TPL_TOKEN: &str =
        "25A1pSwLHvagx8FD3oyAGot1Kfp9keqFhdfGgDZq4s9xjkPc4h5R3P6ikf5ookcsKuZEJDcFShsa3JdgVXYbmgRx";

    #[test]
    fn test_parse_tpl_token_instruction() {
        let rpc_client =
            RpcClient::new_with_commitment(DEFAULT_LOCAL_ENDPOINT, CommitmentConfig::confirmed());
        let signature = Signature::from_str(TEST_SIGNATURE_TPL_TOKEN).unwrap();
        let transaction = rpc_client
            .get_transaction(&signature, UiTransactionEncoding::JsonParsed)
            .unwrap();
        let analyzer = TransactionAnalyzer::new(&transaction);
        let instructions = analyzer.strip_instructions().unwrap();
        assert_eq!(instructions.len(), 1);

        let ix0 = instructions.get(0).unwrap();
        let parsed_ix = parse_instruction(ix0).unwrap();
        if let Instruction::SplToken(detail) = parsed_ix {
            assert_eq!(
                detail.source.to_string(),
                "3DTmFGM7GsH7MJvSkJ8deubVBr46L6tgUcA3XveUMz9L"
            );
            assert_eq!(
                detail.destination.to_string(),
                "7My8xLpS8Nuao32SZ3PsiU9jERNuoWDBtQDrtTKb3guY"
            );
            assert_eq!(detail.amount, 1000);
        } else {
            assert!(false);
        }
    }

    const TEST_SIGNATURE_SYSTEM: &str =
        "4btfTfrW1DSfM9khK25sG53gmczBhPB3bgTAfdCnvyhoiwYju8k5325HhqjmrHfJxuErez4NeyWH5CARsFuAaKpd";

    #[test]
    fn test_parse_system_instruction() {
        let rpc_client =
            RpcClient::new_with_commitment(DEFAULT_LOCAL_ENDPOINT, CommitmentConfig::confirmed());
        let signature = Signature::from_str(TEST_SIGNATURE_SYSTEM).unwrap();
        let transaction = rpc_client
            .get_transaction(&signature, UiTransactionEncoding::JsonParsed)
            .unwrap();
        let analyzer = TransactionAnalyzer::new(&transaction);
        let instructions = analyzer.strip_instructions().unwrap();
        assert_eq!(instructions.len(), 1);

        let ix0 = instructions.get(0).unwrap();
        let parsed_ix = parse_instruction(ix0).unwrap();
        if let Instruction::Solana(detail) = parsed_ix {
            assert_eq!(
                detail.source.to_string(),
                "Afa4Jc8cGhyQc6v64sVw7qpUMiHDrTSc2umPwEdvAZ9M"
            );
            assert_eq!(
                detail.destination.to_string(),
                "dWC1R5jgKfjH79qv4jANoL1Q6FcKGQLYGzRAbqYoqtc"
            );
            assert_eq!(detail.amount, 30000000);
        } else {
            assert!(false);
        }
    }
}
