#![cfg(test)]

use super::mock::*;
use super::*;
use frame_support::{assert_err, assert_ok};

#[test]
fn set_get_address() {
    new_test_ext().execute_with(|| {
        assert_ok!(Bridge::set_address(Origin::ROOT, vec![1, 2, 3, 4]));
        assert_eq!(<EmitterAddress>::get(), vec![1, 2, 3, 4])
    })
}

#[test]
fn asset_transfer_success() {
    new_test_ext().execute_with(|| {
        let chain_id = vec![1];
        let to = vec![2];
        let token_id = vec![3];
        let metadata = vec![];

        assert_ok!(Bridge::whitelist_chain(Origin::ROOT, chain_id.clone()));
        assert_ok!(Bridge::receive_asset(
            Origin::ROOT,
            chain_id.clone(),
            to.clone(),
            token_id.clone(),
            metadata.clone()
        ));
        expect_event(RawEvent::AssetTransfer(
            chain_id.clone(),
            1,
            to.clone(),
            token_id.clone(),
            metadata.clone(),
        ));
    })
}

#[test]
fn asset_transfer_invalid_chain() {
    new_test_ext().execute_with(|| {
        let chain_id = vec![1];
        let to = vec![2];
        let bad_dest_id = vec![3];
        let token_id = vec![4];
        let metadata = vec![];

        assert_ok!(Bridge::whitelist_chain(Origin::ROOT, chain_id));
        assert_err!(
            Bridge::receive_asset(Origin::ROOT, bad_dest_id, to, token_id, metadata),
            Error::<Test>::ChainNotWhitelisted
        );
    })
}

#[test]
fn transfer() {
    new_test_ext_endowed().execute_with(|| {
        // Check inital state
        assert_eq!(<EndowedAccount<Test>>::get(), ENDOWED_ID);
        assert_eq!(Balances::free_balance(&ENDOWED_ID), ENDOWED_BALANCE);
        // Transfer and check result
        assert_ok!(Bridge::transfer(Origin::ROOT, 2, 10));
        assert_eq!(Balances::free_balance(&ENDOWED_ID), ENDOWED_BALANCE - 10);
        assert_eq!(Balances::free_balance(2), 10);
    })
}
