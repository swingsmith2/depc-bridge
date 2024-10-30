use super::{Address, Error};
use crate::bridge::DepcScriptData;
use log::error;

pub fn extract_string_from_script_hex(hex_str: &str) -> Result<DepcScriptData<Address>, Error> {
    //TODO:2. As shown in Figures 2 and 3, implement extract_string_from_script_hex to return in the format of the struct DepcScriptData. The deposit direction only includes the recipient (which is the Solana receiving address specified by the user), while the withdraw direction includes both the recipient and the signature (which is a special request transaction initiated by the user on the DePINC chain with an amount of 0, including the signature of the new transaction on the Solana chain and the target address for withdrawal on the DePINC chain)."
    let data = match hex::decode(hex_str) {
        Ok(r) => r,
        Err(_) => {
            return Err(Error::InvalidHex);
        }
    };

    // check the first byte is OP_RETURN
    const DEFAULT_OPCODE: u8 = 0;
    let opcode = data.get(0).unwrap_or(&DEFAULT_OPCODE);
    if *opcode != OP_RETURN {
        return Err(Error::NotOPReturn);
    }

    // now decode and check the size of the content
    let size = u32::from_le_bytes(data[2..=5].try_into().unwrap()) as usize;
    if size - 1 != data.len() - 6 {
        return Err(Error::InvalidScript);
    }

    Ok(decode_script_after_op_return(&data[6..])?)
}

const OP_RETURN: u8 = 0x6au8;
const OP_PUSHDATA1: u8 = 0x4cu8;
const OP_PUSHDATA2: u8 = 0x4du8;
const OP_PUSHDATA4: u8 = 0x4eu8;

fn decode_script_after_op_return(script: &[u8]) -> Result<DepcScriptData<Address>, Error> {
    let opcode = *match script.get(0) {
        Some(c) => c,
        None => {
            return Err(Error::InvalidScript);
        }
    };
    let mut size = 1usize;
    let mut start_index = 1usize;
    if opcode < OP_PUSHDATA1 {
        size = opcode as usize;
        start_index = 1;
    } else if opcode == OP_PUSHDATA1 {
        size = match script.get(1) {
            Some(n) => *n as usize,
            None => {
                return Err(Error::InvalidScript);
            }
        };
        start_index = 2;
    } else if opcode == OP_PUSHDATA2 {
        if script.len() < 3 {
            return Err(Error::InvalidScript);
        }
        let slice = &script[1..=2];
        size = u16::from_le_bytes(slice.try_into().unwrap()) as usize;
        start_index = 3;
    } else if opcode == OP_PUSHDATA4 {
        if script.len() < 5 {
            return Err(Error::InvalidScript);
        }
        let slice = &script[1..=4];
        size = u32::from_le_bytes(slice.try_into().unwrap()) as usize;
        start_index = 5;
    }
    // ensure the length of slice equals to the number of size which is calculated from above
    let slice = &script[start_index..];
    assert_eq!(slice.len(), size);
    // Ok(match std::str::from_utf8(&slice) {
    //     Ok(s) => s,
    //     Err(_) => {
    //         return Err(Error::InvalidStringFromScript);
    //     }
    // })
    let script: DepcScriptData<Address>;

    script = DepcScriptData {
        recipient: "".parse().unwrap(),
        signature: "".parse().unwrap(),
    };
    Ok(script)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        const HEX: &str = "6a04130000001168656c6c6f20776f726c6420616761696e";
        let s = extract_string_from_script_hex(HEX).unwrap();
        assert_eq!(s, "hello world again");
    }
}
