#![cfg(test)]

use super::mock::{
    new_test_ext, expect_event, Balances, Bridge, Example, Origin, Test, Event, Call,
    ENDOWED_BALANCE, ENDOWED_ID, USER, VALIDATOR_A, VALIDATOR_B, VALIDATOR_C,
};
use super::*;
use frame_support::{assert_noop, assert_ok};

use sp_core::{blake2_256, H256};
use codec::Encode;


#[test]
fn transfer_hash() {
    new_test_ext(2).execute_with(|| {
        let dest_chain = vec![1];
        let token_id = vec![1];
        let hash: H256 = "ABC".using_encoded(blake2_256).into();
        let recipient = vec![99];

        assert_ok!(Bridge::whitelist_chain(Origin::ROOT, dest_chain.clone()));
        assert_ok!(Example::transfer_hash(Origin::signed(1), hash.clone(), recipient.clone()));

        expect_event(bridge::RawEvent::AssetTransfer(dest_chain, 1, recipient, vec![1], hash.as_ref().to_vec()));
    })
}

fn make_proposal(hash: H256) -> Call {
    Call::Example(crate::Call::remark(hash))
}