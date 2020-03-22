#![cfg(test)]

use super::mock::{
    expect_event, new_test_ext, Balances, Bridge, Call, Event, Example, Origin, Test,
    VALIDATOR_A, VALIDATOR_B, VALIDATOR_C,
};
use super::*;
use frame_support::dispatch::DispatchError;
use frame_support::{assert_noop, assert_ok};

use codec::Encode;
use sp_core::{blake2_256, H256};

#[test]
fn transfer_hash() {
    new_test_ext(2).execute_with(|| {
        let dest_chain = vec![1];
        let token_id = vec![1];
        let hash: H256 = "ABC".using_encoded(blake2_256).into();
        let recipient = vec![99];

        assert_ok!(Bridge::whitelist_chain(Origin::ROOT, dest_chain.clone()));
        assert_ok!(Example::transfer_hash(
            Origin::signed(1),
            hash.clone(),
            recipient.clone()
        ));

        expect_event(bridge::RawEvent::AssetTransfer(
            dest_chain,
            1,
            recipient,
            token_id,
            hash.as_ref().to_vec(),
        ));
    })
}

fn make_proposal(hash: H256) -> Call {
    Call::Example(crate::Call::remark(hash))
}

#[test]
fn execute_remark() {
    new_test_ext(2).execute_with(|| {
        let hash: H256 = "ABC".using_encoded(blake2_256).into();
        let proposal = make_proposal(hash.clone());
        let prop_id = 1;

        assert_ok!(Bridge::create_proposal(
            Origin::signed(VALIDATOR_A),
            prop_id,
            Box::new(proposal.clone())
        ));
        assert_ok!(Bridge::approve(
            Origin::signed(VALIDATOR_B),
            prop_id,
            Box::new(proposal.clone())
        ));

        expect_event(RawEvent::Remark(hash))
    })
}

#[test]
fn execute_remark_bad_origin() {
    new_test_ext(2).execute_with(|| {
        let hash: H256 = "ABC".using_encoded(blake2_256).into();

        assert_ok!(Example::remark(Origin::signed(Bridge::account_id()), hash));
        // Don't allow any signed origin except from bridge addr
        assert_noop!(
            Example::remark(Origin::signed(VALIDATOR_A), hash),
            DispatchError::BadOrigin
        );
        // Don't allow root calls
        assert_noop!(
            Example::remark(Origin::ROOT, hash),
            DispatchError::BadOrigin
        );
    })
}
