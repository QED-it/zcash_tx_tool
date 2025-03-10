use crate::components::transactions::{create_burn_transaction, create_transfer_transaction};
use crate::components::user::User;
use crate::prelude::info;
use orchard::keys::Scope::External;
use orchard::note::AssetBase;
use zcash_primitives::transaction::Transaction;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct TestBalances(Vec<i64>);

impl TestBalances {
    pub(crate) fn add_balances(&mut self, balances: Vec<(u32, i64)>) {
        for (index, balance) in balances {
            assert!((index as usize) < self.0.len());
            self.0[index as usize] += balance;
        }
    }

    pub(crate) fn get_zec(user: &mut User, num_users: u32) -> TestBalances {
        Self::get_asset(AssetBase::native(), user, num_users)
    }

    pub(crate) fn get_asset(asset: AssetBase, wallet: &mut User, num_users: u32) -> TestBalances {
        let mut balance_vec = vec![];
        for i in 0..num_users {
            let address = wallet.address_for_account(i, External);
            let balance = wallet.balance(address, asset) as i64;
            balance_vec.push(balance);
        }

        TestBalances(balance_vec)
    }
}

pub(crate) struct TransferInfo {
    index_from: u32,
    index_to: u32,
    amount: u64,
}

impl TransferInfo {
    pub(crate) fn new(index_from: u32, index_to: u32, amount: u64) -> Self {
        TransferInfo {
            index_from,
            index_to,
            amount,
        }
    }
    pub(crate) fn create_transfer_txn(&self, asset: AssetBase, wallet: &mut User) -> Transaction {
        let from_addr = wallet.address_for_account(self.index_from, External);
        let to_addr = wallet.address_for_account(self.index_to, External);
        create_transfer_transaction(from_addr, to_addr, self.amount, asset, wallet)
    }
}

pub(crate) struct BurnInfo {
    index: u32,
    amount: u64,
}

impl BurnInfo {
    pub(crate) fn new(index: u32, amount: u64) -> Self {
        BurnInfo { index, amount }
    }

    pub(crate) fn create_burn_txn(&self, asset: AssetBase, wallet: &mut User) -> Transaction {
        let addr = wallet.address_for_account(self.index, External);
        create_burn_transaction(addr, self.amount, asset, wallet)
    }
}

pub(crate) fn update_balances_after_transfer(
    balances: &TestBalances,
    transfer_info_vec: &Vec<TransferInfo>,
) -> TestBalances {
    let mut new_balances = balances.clone();
    for transfer_info in transfer_info_vec {
        new_balances.0[transfer_info.index_from as usize] -= transfer_info.amount as i64;
        new_balances.0[transfer_info.index_to as usize] += transfer_info.amount as i64;
    }
    new_balances
}

pub(crate) fn update_balances_after_burn(
    balances: &TestBalances,
    burn_vec: &Vec<BurnInfo>,
) -> TestBalances {
    let mut new_balances = balances.clone();
    for burn_info in burn_vec {
        new_balances.0[burn_info.index as usize] -= burn_info.amount as i64;
    }
    new_balances
}

pub(crate) fn check_balances(
    header: &str,
    asset: AssetBase,
    expected_balances: TestBalances,
    user: &mut User,
    num_users: u32,
) {
    let actual_balances = TestBalances::get_asset(asset, user, num_users);
    print_balances(header, asset, &actual_balances);
    assert_eq!(actual_balances, expected_balances);
}

pub(crate) fn print_balances(header: &str, asset: AssetBase, balances: &TestBalances) {
    info!("{}", header);
    if asset.is_native().into() {
        info!("AssetBase: Native ZEC");
    } else {
        let trimmed_asset_base = hex::encode(asset.to_bytes())
            .as_str()
            .chars()
            .take(8)
            .collect::<String>();
        info!("AssetBase: {}", trimmed_asset_base);
    }
    for (i, balance) in balances.0.iter().enumerate() {
        info!("Account {} balance: {}", i, balance);
    }
}
