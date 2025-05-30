use crate::components::user::User;
use crate::prelude::info;
use orchard::keys::Scope::External;
use orchard::note::AssetBase;

#[derive(Debug, Copy, Clone)]
pub(crate) struct TestBalances {
    account0: i64,
    account1: i64,
}

impl TestBalances {
    pub(crate) fn new(account0: i64, account1: i64) -> Self {
        TestBalances { account0, account1 }
    }

    pub(crate) fn get_zec(user: &mut User) -> TestBalances {
        Self::get_asset(AssetBase::native(), user)
    }

    pub(crate) fn get_asset(asset: AssetBase, wallet: &mut User) -> TestBalances {
        let address0 = wallet.address_for_account(0, External);
        let address1 = wallet.address_for_account(1, External);

        let balance0 = wallet.balance(address0, asset) as i64;
        let balance1 = wallet.balance(address1, asset) as i64;

        TestBalances {
            account0: balance0,
            account1: balance1,
        }
    }
}

pub(crate) fn check_balances(
    header: &str,
    asset: AssetBase,
    initial: TestBalances,
    expected_delta: TestBalances,
    user: &mut User,
) -> TestBalances {
    let actual_balances = TestBalances::get_asset(asset, user);
    print_balances(header, asset, actual_balances);
    assert_eq!(
        actual_balances.account0,
        initial.account0 + expected_delta.account0
    );
    assert_eq!(
        actual_balances.account1,
        initial.account1 + expected_delta.account1
    );
    actual_balances
}

pub(crate) fn print_balances(header: &str, asset: AssetBase, balances: TestBalances) {
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
    info!("Account 0 balance: {}", balances.account0);
    info!("Account 1 balance: {}", balances.account1);
}
