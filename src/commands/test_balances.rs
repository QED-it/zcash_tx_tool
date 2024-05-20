use orchard::keys::Scope::External;
use orchard::note::AssetBase;
use crate::components::wallet::Wallet;
use crate::prelude::info;

#[derive(Debug, Copy, Clone)]
pub(crate) struct TestBalances {
    account0: i64,
    account1: i64,
}

impl TestBalances {

    pub(crate) fn new(account0: i64, account1: i64) -> Self {
        TestBalances { account0, account1 }
    }

    pub(crate) fn get_zec(wallet: &mut Wallet) -> TestBalances {
        Self::get_asset(AssetBase::native(), wallet)
    }

    pub(crate) fn get_asset(asset: AssetBase, wallet: &mut Wallet) -> TestBalances {
        TestBalances {
            account0: wallet.balance(wallet.address_for_account(0, External), asset) as i64,
            account1: wallet.balance(wallet.address_for_account(1, External), asset) as i64,
        }
    }
}

pub(crate) fn check_balances(header: &str, initial: TestBalances, expected_delta: TestBalances, wallet: &mut Wallet) -> TestBalances {
    let actual_balances = TestBalances::get_zec(wallet);
    print_balances(header, actual_balances);
    assert_eq!(actual_balances.account0, initial.account0 + expected_delta.account0);
    assert_eq!(actual_balances.account1, initial.account1 + expected_delta.account1);
    actual_balances
}

pub(crate) fn print_balances(header: &str, balances: TestBalances) {
    info!("{}", header);
    info!("Account 0 balance: {}", balances.account0);
    info!("Account 1 balance: {}", balances.account1);
}