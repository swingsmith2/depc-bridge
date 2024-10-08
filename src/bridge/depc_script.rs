pub struct Deposit {
    pub depc_txid: String,
    pub from_depc_address: String,
    pub to_erc20_address: String,
    pub total: u64,
}

#[derive(Debug)]
pub enum Error {
    InvalidHex,
    InvalidScript,
    NotOPReturn,
    InvalidStringFromScript,
    NotErc20Address,
}

pub fn extract_string_from_script_hex(hex_str: &str) -> Result<String, Error> {
    let data = match hex::decode(hex_str) {
        Ok(r) => r,
        Err(e) => {
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

    Ok(decode_script_after_op_return(&data[6..])?.to_owned())
}

const OP_RETURN: u8 = 0x6au8;
const OP_PUSHDATA1: u8 = 0x4cu8;
const OP_PUSHDATA2: u8 = 0x4du8;
const OP_PUSHDATA4: u8 = 0x4eu8;

fn decode_script_after_op_return(script: &[u8]) -> Result<&str, Error> {
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
    Ok(match std::str::from_utf8(&slice) {
        Ok(s) => s,
        Err(_) => {
            return Err(Error::InvalidStringFromScript);
        }
    })
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