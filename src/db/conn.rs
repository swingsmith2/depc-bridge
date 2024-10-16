use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection, Error};

const SQL_BEGIN_TRANSACTION: &str = "begin transaction";

const SQL_ROLLBACK_TRANSACTION: &str = "rollback transaction";

const SQL_COMMIT_TRANSACTION: &str = "commit transaction";

/// Table `blocks`
const SQL_CREATE_TABLE_BLOCKS: &str =
    "create table if not exists blocks (hash, height, miner, time)";
const SQL_CREATE_UNIQUE_INDEX_BLOCKS_HASH: &str =
    "create unique index if not exists index__blocks_hash on blocks (hash)";
const SQL_INSERT_BLOCK: &str = "insert into blocks (hash, height, miner, time) values (?, ?, ?, ?)";

/// Table `transactions`
const SQL_CREATE_TABLE_TRANSACTIONS: &str =
    "create table if not exists transactions (block_hash, txid)";
const SQL_CREATE_UNIQUE_INDEX_TRANSACTIONS_TXID: &str =
    "create unique index if not exists index__transactions_txid on transactions (txid)";
const SQL_INSERT_TRANSACTION: &str = "insert into transactions (block_hash, txid) values (?, ?)";

/// Table `coins`
const SQL_CREATE_TABLE_COINS: &str =
    "create table if not exists coins (txid, n, value, owner, script_hex, is_spent, spent_height, spent_txid)";
const SQL_CREATE_UNIQUE_INDEX_COINS_TXID_N: &str =
    "create unique index if not exists index__coins_txid_n on coins (txid, n)";
const SQL_CREATE_INDEX_COINS_SPENT_TXID: &str =
    "create index if not exists index__coins_spent_txid on coins (spent_txid)";
const SQL_CREATE_INDEX_COINS_OWNER: &str =
    "create index if not exists index__coins_owner on coins (owner)";
const SQL_CREATE_INDEX_COINS_SPENT_HEIGHT: &str =
    "create index if not exists index__coins_spent_height on coins (spent_height)";
const SQL_INSERT_COIN: &str =
    "insert into coins (txid, n, value, owner, script_hex, is_spent) values (?, ?, ?, ?, ?, ?)";
const SQL_MARK_COIN_SPENT: &str =
    "update coins set is_spent = true, spent_txid = ?, spent_height = ? where txid = ? and n = ?";

/// Table `deposit`
/// the reson I removed `from_address_depc` is because it's a bit more complex of the UTXO model,
/// A transaction might contains more than one incoming addresses. We might need to create
/// a slave table contains the addresses which are related to a deposit.
const SQL_CREATE_TABLE_DEPC_DEPOSIT: &str = "create table if not exists depc_deposit (depc_txid, depc_timestamp, to_address_erc20, amount, erc20_txid, erc20_timestamp)";
const SQL_CREATE_UNIQUE_INDEX_DEPC_DEPOSIT_DEPC_TXID: &str =
    "create unique index if not exists index__depc_deposit_depc_txid on depc_deposit (depc_txid)";
const SQL_INSERT_DEPC_DEPOSIT: &str = "insert into depc_deposit (depc_txid, to_address_erc20, amount, depc_timestamp) values (?, ?, ?, ?)";
const SQL_UPDATE_DEPC_DEPSOIT: &str =
    "update depc_deposit set erc20_txid = ?, erc20_timestamp = ? where depc_txid = ?";

/// Table `withdraw`
const SQL_CREATE_TABLE_DEPC_WITHDRAW: &str = "create table if not exists depc_withdraw (erc20_txid, erc20_timestamp, from_address_erc20, to_address_depc, amount, depc_txid, depc_timestamp)";
const SQL_CREATE_UNIQUE_INDEX_DEPC_WITHDRAW_ERC20_TXID: &str = "create unique index if not exists index__depc_withdraw_erc20_txid on depc_withdraw (erc20_txid)";
const SQL_INSERT_DEPC_WITHDRAW: &str = "insert into depc_withdraw (erc20_txid, erc20_timestamp, from_address_erc20, amount) values (?, ?, ?, ?)";
const SQL_UPDATE_DEPC_WITHDRAW: &str =
    "update depc_withdraw set depc_txid = ?, depc_timestamp = ?, to_address_depc = ? where erc20_txid = ?";
const SQL_QUERY_BEST_HEIGHT: &str = "select height from blocks order by height desc limit 1";
const SQL_QUERY_ADDRESSES_FROM_TX_INPUTS: &str =
    "select owner from coins where spent_txid = ? and is_spent = true";
const SQL_QUERY_TXIDS_THOSE_INPUTS_CONTAIN_ADDRESS: &str =
    "select spent_txid from coins where owner = ? and is_spent = true group by spent_txid";
const SQL_QUERY_BALANCE_OF_ADDRESS: &str =
    "select sum(value) from coins left join transactions on transactions.txid = coins.txid left join blocks on blocks.hash = transactions.block_hash where owner = ? and height <= ? and (spent_height is null or spent_height > ?)";

const SQL_QUERY_BLOCK_TIME_BY_HEIGHT: &str = "select time from blocks where height = ?";

/// Table `exchange_addresses`
const SQL_CREATE_TABLE_EXCHANGE_ADDRESSES: &str = "create table if not exists exchange_addresses (address text primary key not null, analyzed_txid text not null)";
const SQL_CREATE_INDEX_EXCHANGE_ADDRESSES_ANALYZED_TXID: &str = "create index if not exists index__exchange_addresses_analyzed_txid on exchange_addresses (analyzed_txid)";
const SQL_INSERT_EXCHANGE_ADDRESSE: &str =
    "insert into exchange_addresses (address, analyzed_txid) values (?, ?)";
const SQL_QUERY_EXCHANGE_ADDRESSES: &str = "select address from exchange_addresses";
const SQL_QUERY_NUM_EXCHANGE_ADDRESSES: &str = "select count(*) from exchange_addresses";

#[derive(Clone)]
pub struct Conn {
    conn: Arc<Mutex<Connection>>,
}

impl Conn {
    pub fn open_or_create(db_path: &str) -> Result<Conn, Error> {
        let conn = Connection::open(db_path)?;
        Ok(Conn {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    #[cfg(test)]
    pub fn open_in_mem() -> Result<Conn, Error> {
        let conn = Connection::open_in_memory()?;
        Ok(Conn {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn init(&self) -> Result<(), Error> {
        let c = self.conn.lock().unwrap();
        c.execute(SQL_CREATE_TABLE_BLOCKS, [])?;
        c.execute(SQL_CREATE_UNIQUE_INDEX_BLOCKS_HASH, [])?;

        c.execute(SQL_CREATE_TABLE_TRANSACTIONS, [])?;
        c.execute(SQL_CREATE_UNIQUE_INDEX_TRANSACTIONS_TXID, [])?;

        c.execute(SQL_CREATE_TABLE_COINS, [])?;
        c.execute(SQL_CREATE_UNIQUE_INDEX_COINS_TXID_N, [])?;
        c.execute(SQL_CREATE_INDEX_COINS_SPENT_TXID, [])?;
        c.execute(SQL_CREATE_INDEX_COINS_OWNER, [])?;
        c.execute(SQL_CREATE_INDEX_COINS_SPENT_HEIGHT, [])?;

        c.execute(SQL_CREATE_TABLE_DEPC_DEPOSIT, [])?;
        c.execute(SQL_CREATE_UNIQUE_INDEX_DEPC_DEPOSIT_DEPC_TXID, [])?;

        c.execute(SQL_CREATE_TABLE_DEPC_WITHDRAW, [])?;
        c.execute(SQL_CREATE_UNIQUE_INDEX_DEPC_WITHDRAW_ERC20_TXID, [])?;

        c.execute(SQL_CREATE_TABLE_EXCHANGE_ADDRESSES, [])?;
        c.execute(SQL_CREATE_INDEX_EXCHANGE_ADDRESSES_ANALYZED_TXID, [])?;

        Ok(())
    }

    pub fn begin_transaction(&self) -> Result<(), Error> {
        let c = self.conn.lock().unwrap();
        c.execute(SQL_BEGIN_TRANSACTION, [])?;
        Ok(())
    }

    pub fn rollback_transaction(&self) -> Result<(), Error> {
        let c = self.conn.lock().unwrap();
        c.execute(SQL_ROLLBACK_TRANSACTION, [])?;
        Ok(())
    }

    pub fn commit_transaction(&self) -> Result<(), Error> {
        let c = self.conn.lock().unwrap();
        c.execute(SQL_COMMIT_TRANSACTION, [])?;
        Ok(())
    }

    pub fn add_block(&self, hash: &str, height: u32, miner: &str, time: u64) -> Result<(), Error> {
        let c = self.conn.lock().unwrap();
        c.execute(SQL_INSERT_BLOCK, params![hash, height, miner, time])?;
        Ok(())
    }

    pub fn add_transaction(&self, block_hash: &str, txid: &str) -> Result<(), Error> {
        let c = self.conn.lock().unwrap();
        c.execute(SQL_INSERT_TRANSACTION, params![block_hash, txid])?;
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
        let c = self.conn.lock().unwrap();
        c.execute(
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
        let c = self.conn.lock().unwrap();
        c.execute(
            SQL_MARK_COIN_SPENT,
            params![spent_txid, spent_height, txid, n],
        )?;
        Ok(())
    }

    pub fn make_deposit(
        &self,
        depc_txid: &str,
        to_address_erc20: &str,
        amount: u64,
        depc_timestamp: u64,
    ) -> Result<(), Error> {
        let c = self.conn.lock().unwrap();
        c.execute(
            SQL_INSERT_DEPC_DEPOSIT,
            params![depc_txid, to_address_erc20, amount, depc_timestamp],
        )?;
        Ok(())
    }

    pub fn confirm_deposit(
        &self,
        erc20_txid: &str,
        erc20_timestamp: u64,
        depc_txid: &str,
    ) -> Result<(), Error> {
        let c = self.conn.lock().unwrap();
        c.execute(
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
        amount: u64,
    ) -> Result<(), Error> {
        let c = self.conn.lock().unwrap();
        c.execute(
            SQL_INSERT_DEPC_WITHDRAW,
            params![erc20_txid, erc20_timestamp, from_address_erc20, amount],
        )?;
        Ok(())
    }

    pub fn confirm_withdraw(
        &self,
        depc_txid: &str,
        depc_timestamp: u64,
        depc_address: &str,
        erc20_txid: &str,
    ) -> Result<(), Error> {
        let c = self.conn.lock().unwrap();
        c.execute(
            SQL_UPDATE_DEPC_WITHDRAW,
            params![depc_txid, depc_timestamp, depc_address, erc20_txid],
        )?;
        Ok(())
    }

    pub fn query_best_height(&self) -> Option<u32> {
        let c = self.conn.lock().unwrap();
        if let Ok(height) = c.query_row(SQL_QUERY_BEST_HEIGHT, [], |row| -> Result<u32, Error> {
            let height = row.get(0).unwrap();
            Ok(height)
        }) {
            Some(height)
        } else {
            None
        }
    }

    pub fn query_block_time_by_height(&self, height: u32) -> u64 {
        let c = self.conn.lock().unwrap();
        c.query_row(SQL_QUERY_BLOCK_TIME_BY_HEIGHT, params![height], |row| {
            row.get(0)
        })
        .unwrap()
    }

    pub fn query_balance(&self, address: &str, height: u32) -> Result<u64, Error> {
        let c = self.conn.lock().unwrap();
        Ok(c.query_row(
            SQL_QUERY_BALANCE_OF_ADDRESS,
            params![address, height, height],
            |row| row.get(0),
        )?)
    }

    pub fn query_inputs(&self, txid: &str) -> Result<Vec<String>, Error> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(SQL_QUERY_ADDRESSES_FROM_TX_INPUTS)?;
        let iter = stmt.query_map(params![txid], |row| {
            let address: String = row.get(0)?;
            Ok(address)
        })?;
        iter.collect()
    }

    pub fn query_txids_those_inputs_contain_address(
        &self,
        address: &str,
    ) -> Result<Vec<String>, Error> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(SQL_QUERY_TXIDS_THOSE_INPUTS_CONTAIN_ADDRESS)?;
        let iter = stmt.query_map(params![address], |row| Ok(row.get(0).unwrap()))?;
        iter.collect()
    }

    pub fn add_analyzed_exchange_address_from_tx(
        &self,
        address: &str,
        txid: &str,
    ) -> Result<(), Error> {
        let c = self.conn.lock().unwrap();
        c.execute(SQL_INSERT_EXCHANGE_ADDRESSE, params![address, txid])?;
        Ok(())
    }

    pub fn query_analyzed_exchange_addresses(&self) -> Result<Vec<String>, Error> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(SQL_QUERY_EXCHANGE_ADDRESSES)?;
        let iter = stmt.query_map([], |row| {
            let address: String = row.get(0)?;
            Ok(address)
        })?;
        iter.collect()
    }

    pub fn query_num_exchange_addresses(&self) -> Result<u64, Error> {
        let c = self.conn.lock().unwrap();
        Ok(c.query_row(SQL_QUERY_NUM_EXCHANGE_ADDRESSES, [], |row| row.get(0))?)
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

        conn.make_deposit("depc_txid", "to_erc20_address", 10000000, 394838121)
            .unwrap();

        conn.confirm_deposit("erc20_txid", 193847845, "depc_txid")
            .unwrap();
    }

    #[test]
    fn test_make_withdraw() {
        let conn = Conn::open_in_mem().unwrap();
        conn.init().unwrap();

        conn.make_withdraw("erc20_txid", 193847845, "from_address", 1000000)
            .unwrap();
        conn.confirm_withdraw("depc_txid", 193848478, "erc20_txid", "depc_address")
            .unwrap();
    }
}
