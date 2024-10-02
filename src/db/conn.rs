use rusqlite::{params, Connection, Error};

const SQL_CREATE_TABLE_BLOCKS: &str =
    "create table if not exists blocks (hash, height, miner, time)";
const SQL_CREATE_UNIQUE_INDEX_BLOCKS_HASH: &str =
    "create unique index if not exists index__blocks_hash on blocks (hash)";
const SQL_INSERT_BLOCK: &str = "insert into blocks (hash, height, miner, time) values (?, ?, ?, ?)";

const SQL_CREATE_TABLE_TRANSACTIONS: &str =
    "create table if not exists transactions (block_hash, txid)";
const SQL_CREATE_UNIQUE_INDEX_TRANSACTIONS_TXID: &str =
    "create unique index index__transactions_txid on transactions (txid)";
const SQL_INSERT_TRANSACTION: &str = "insert into transactions (block_hash, txid) values (?, ?)";

const SQL_CREATE_TABLE_COINS: &str =
    "create table if not exists coins (txid, n, value, owner, script_hex, is_spent, spent_height, spent_txid)";
const SQL_CREATE_UNIQUE_INDEX_COINS_TXID_N: &str =
    "create unique index if not exists index__coins_txid_n on coins (txid, n)";
const SQL_INSERT_COIN: &str =
    "insert into coins (txid, n, value, owner, script_hex, is_spent) values (?, ?, ?, ?, ?, ?)";
const SQL_MARK_COIN_SPENT: &str =
    "update coins set is_spent = true, spent_txid = ?, spent_height = ? where txid = ? and n = ?";

const SQL_CREATE_TABLE_DEPC_DEPOSIT: &str = "create table if not exists depc_deposit (depc_txid, depc_timestamp, from_address_depc, to_address_erc20, amount, erc20_txid, erc20_timestamp)";
const SQL_CREATE_UNIQUE_INDEX_DEPC_DEPOSIT_DEPC_TXID: &str =
    "create unique index if not exists index__depc_deposit_depc_txid on depc_deposit (depc_txid)";
const SQL_INSERT_DEPC_DEPOSIT: &str = "insert into depc_deposit (depc_txid, from_address_depc, to_address_erc20, amount, depc_timestamp) values (?, ?, ?, ?, ?)";
const SQL_UPDATE_DEPC_DEPSOIT: &str =
    "update depc_deposit set erc20_txid = ?, erc20_timestamp = ? where depc_txid = ?";

const SQL_CREATE_TABLE_DEPC_WITHDRAW: &str = "create table if not exists depc_withdraw (erc20_txid, erc20_timestamp, from_address_erc20, to_address_depc, amount, depc_txid, depc_timestamp)";
const SQL_CREATE_UNIQUE_INDEX_DEPC_WITHDRAW_ERC20_TXID: &str = "create unique index if not exists index__depc_withdraw_erc20_txid on depc_withdraw (erc20_txid)";
const SQL_INSERT_DEPC_WITHDRAW: &str = "insert into depc_withdraw (erc20_txid, erc20_timestamp, from_address_erc20, to_address_depc, amount) values (?, ?, ?, ?, ?)";
const SQL_UPDATE_DEPC_WITHDRAW: &str =
    "update depc_withdraw set depc_txid = ?, depc_timestamp = ? where erc20_txid = ?";
const SQL_QUERY_BEST_HEIGHT: &str = "select height from blocks order by height desc limit 1";

pub struct Conn {
    conn: Connection,
}

impl Conn {
    pub fn open_or_create(db_path: &str) -> Result<Conn, Error> {
        let conn = Connection::open(db_path)?;
        Ok(Conn { conn })
    }

    pub fn open_in_mem() -> Result<Conn, Error> {
        let conn = Connection::open_in_memory()?;
        Ok(Conn { conn })
    }

    pub fn init(&self) -> Result<(), Error> {
        self.conn.execute(SQL_CREATE_TABLE_BLOCKS, [])?;
        self.conn.execute(SQL_CREATE_UNIQUE_INDEX_BLOCKS_HASH, [])?;

        self.conn.execute(SQL_CREATE_TABLE_TRANSACTIONS, [])?;
        self.conn
            .execute(SQL_CREATE_UNIQUE_INDEX_TRANSACTIONS_TXID, [])?;

        self.conn.execute(SQL_CREATE_TABLE_COINS, [])?;
        self.conn
            .execute(SQL_CREATE_UNIQUE_INDEX_COINS_TXID_N, [])?;

        self.conn.execute(SQL_CREATE_TABLE_DEPC_DEPOSIT, [])?;
        self.conn
            .execute(SQL_CREATE_UNIQUE_INDEX_DEPC_DEPOSIT_DEPC_TXID, [])?;

        self.conn.execute(SQL_CREATE_TABLE_DEPC_WITHDRAW, [])?;
        self.conn
            .execute(SQL_CREATE_UNIQUE_INDEX_DEPC_WITHDRAW_ERC20_TXID, [])?;

        Ok(())
    }

    pub fn add_block(&self, hash: &str, height: u32, miner: &str, time: u64) -> Result<(), Error> {
        self.conn
            .execute(SQL_INSERT_BLOCK, params![hash, height, miner, time])?;
        Ok(())
    }

    pub fn add_transaction(&self, block_hash: &str, txid: &str) -> Result<(), Error> {
        self.conn
            .execute(SQL_INSERT_TRANSACTION, params![block_hash, txid])?;
        Ok(())
    }

    pub fn add_coin(
        &self,
        txid: &str,
        n: u32,
        value: u64,
        owner: &str,
        script_hex: &str,
    ) -> Result<(), Error> {
        self.conn.execute(
            SQL_INSERT_COIN,
            params![txid, n, value, owner, script_hex, false],
        )?;
        Ok(())
    }

    pub fn mark_coin_to_spent(
        &self,
        txid: &str,
        n: u32,
        spent_txid: &str,
        spent_height: u32,
    ) -> Result<(), Error> {
        self.conn.execute(
            SQL_MARK_COIN_SPENT,
            params![spent_txid, spent_height, txid, n],
        )?;
        Ok(())
    }

    pub fn make_deposit(
        &self,
        depc_txid: &str,
        from_address_depc: &str,
        to_address_erc20: &str,
        amount: u64,
        depc_timestamp: u64,
    ) -> Result<(), Error> {
        self.conn.execute(
            SQL_INSERT_DEPC_DEPOSIT,
            params![
                depc_txid,
                from_address_depc,
                to_address_erc20,
                amount,
                depc_timestamp
            ],
        )?;
        Ok(())
    }

    // "update depc_deposit set erc20_txid = ?, erc20_timestamp = ? where depc_txid = ?";
    pub fn confirm_deposit(
        &self,
        erc20_txid: &str,
        erc20_timestamp: u64,
        depc_txid: &str,
    ) -> Result<(), Error> {
        self.conn.execute(
            SQL_UPDATE_DEPC_DEPSOIT,
            params![erc20_txid, erc20_timestamp, depc_txid],
        )?;
        Ok(())
    }

    pub fn make_withdraw(
        &self,
        erc20_txid: &str,
        erc20_timestamp: u64,
        from_address_erc20: &str,
        to_address_depc: &str,
        amount: u64,
    ) -> Result<(), Error> {
        self.conn.execute(
            SQL_INSERT_DEPC_WITHDRAW,
            params![
                erc20_txid,
                erc20_timestamp,
                from_address_erc20,
                to_address_depc,
                amount
            ],
        )?;
        Ok(())
    }

    pub fn confirm_withdraw(
        &self,
        depc_txid: &str,
        depc_timestamp: u64,
        erc20_txid: &str,
    ) -> Result<(), Error> {
        self.conn.execute(
            SQL_UPDATE_DEPC_WITHDRAW,
            params![depc_txid, depc_timestamp, erc20_txid],
        )?;
        Ok(())
    }

    pub fn query_best_height(&self) -> Option<u32> {
        if let Ok(height) =
            self.conn
                .query_row(SQL_QUERY_BEST_HEIGHT, [], |row| -> Result<u32, Error> {
                    let height = row.get(0).unwrap();
                    Ok(height)
                })
        {
            Some(height)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_or_create() {
        assert!(Conn::open_or_create(&shellexpand::env("$HOME/hello.sqlite3").unwrap()).is_ok());
    }

    #[test]
    fn test_open_in_memory_init() {
        let conn = Conn::open_in_mem().unwrap();
        conn.init().unwrap();
    }

    #[test]
    fn test_add_block() {
        let conn = Conn::open_in_mem().unwrap();
        conn.init().unwrap();

        conn.add_block("hash value", 1000, "address", 1938483848)
            .unwrap();
    }

    #[test]
    fn test_add_transaction() {
        let conn = Conn::open_in_mem().unwrap();
        conn.init().unwrap();

        conn.add_transaction("hash value", "txid").unwrap();
    }

    #[test]
    fn test_add_coin_and_mark() {
        let conn = Conn::open_in_mem().unwrap();
        conn.init().unwrap();

        conn.add_coin("txid", 0, 1000, "helloaddress", "39204848b93948")
            .unwrap();
        conn.mark_coin_to_spent("txid", 0, "spent_txid", 10203)
            .unwrap();
    }

    #[test]
    fn test_make_deposit() {
        let conn = Conn::open_in_mem().unwrap();
        conn.init().unwrap();

        conn.make_deposit(
            "depc_txid",
            "from_address",
            "to_erc20_address",
            10000000,
            394838121,
        )
        .unwrap();

        conn.confirm_deposit("erc20_txid", 193847845, "depc_txid")
            .unwrap();
    }

    #[test]
    fn test_make_withdraw() {
        let conn = Conn::open_in_mem().unwrap();
        conn.init().unwrap();

        conn.make_withdraw(
            "erc20_txid",
            193847845,
            "from_address",
            "to_address",
            1000000,
        )
        .unwrap();
        conn.confirm_withdraw("depc_txid", 193848478, "erc20_txid")
            .unwrap();
    }
}