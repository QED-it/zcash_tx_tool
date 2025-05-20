use crate::components::transactions::{create_burn_transaction, create_transfer_transaction};
use crate::components::user::User;
use crate::prelude::info;
use orchard::keys::Scope::External;
use orchard::note::AssetBase;
use zcash_primitives::transaction::Transaction;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct TestBalances(Vec<u64>);

impl TestBalances {
    pub(crate) fn get_native_balances(num_accounts: usize, user: &mut User) -> TestBalances {
        Self::get_asset_balances(AssetBase::native(), num_accounts, user)
    }

    pub(crate) fn get_asset_balances(
        asset: AssetBase,
        num_accounts: usize,
        wallet: &mut User,
    ) -> TestBalances {
        let balances = (0..num_accounts)
            .map(|i| {
                let address = wallet.address_for_account(i, External);
                wallet.balance(address, asset)
            })
            .collect();

        TestBalances(balances)
    }
}

pub(crate) struct TransferInfo {
    acc_idx_from: usize,
    acc_idx_to: usize,
    asset: AssetBase,
    amount: u64,
}

impl TransferInfo {
    pub(crate) fn new(
        acc_idx_from: usize,
        acc_idx_to: usize,
        asset: AssetBase,
        amount: u64,
    ) -> Self {
        TransferInfo {
            acc_idx_from,
            acc_idx_to,
            asset,
            amount,
        }
    }
    pub(crate) fn create_transfer_txn(&self, wallet: &mut User) -> Transaction {
        let from_addr = wallet.address_for_account(self.acc_idx_from, External);
        let to_addr = wallet.address_for_account(self.acc_idx_to, External);
        create_transfer_transaction(from_addr, to_addr, self.amount, self.asset, wallet)
    }
}

pub(crate) struct BurnInfo {
    burner_acc_idx: usize,
    asset: AssetBase,
    amount: u64,
}

impl BurnInfo {
    pub(crate) fn new(burner_acc_idx: usize, asset: AssetBase, amount: u64) -> Self {
        BurnInfo {
            burner_acc_idx,
            asset,
            amount,
        }
    }

    pub(crate) fn create_burn_txn(&self, wallet: &mut User) -> Transaction {
        let address = wallet.address_for_account(self.burner_acc_idx, External);
        create_burn_transaction(address, self.amount, self.asset, wallet)
    }
}

pub(crate) fn expected_balances_after_mine(
    balances: &TestBalances,
    miner_idx: usize,
) -> TestBalances {
    let coinbase_value = 625_000_000;
    let mut new_balances = balances.clone();
    new_balances.0[miner_idx] += coinbase_value;
    new_balances
}
pub(crate) fn expected_balances_after_transfer(
    balances: &TestBalances,
    transfers: &[TransferInfo],
) -> TestBalances {
    let new_balances = transfers
        .iter()
        .fold(balances.clone(), |mut acc, transfer_info| {
            acc.0[transfer_info.acc_idx_from] -= transfer_info.amount;
            acc.0[transfer_info.acc_idx_to] += transfer_info.amount;
            acc
        });
    new_balances
}

pub(crate) fn expected_balances_after_burn(
    balances: &TestBalances,
    burns: &[BurnInfo],
) -> TestBalances {
    let new_balances = burns.iter().fold(balances.clone(), |mut acc, burn_info| {
        acc.0[burn_info.burner_acc_idx] -= burn_info.amount;
        acc
    });
    new_balances
}

pub(crate) fn check_balances(
    asset: AssetBase,
    expected_balances: &TestBalances,
    user: &mut User,
    num_accounts: usize,
) {
    let actual_balances = TestBalances::get_asset_balances(asset, num_accounts, user);
    assert_eq!(&actual_balances, expected_balances);
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
    balances.0.iter().enumerate().for_each(|(i, balance)| {
        info!("Account {} balance: {}", i, balance);
    });
}
