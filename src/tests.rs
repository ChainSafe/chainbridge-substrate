#![cfg(test)]

use super::mock::*;
use super::*;
use frame_support::{assert_ok, assert_noop};
use sp_core::{blake2_256, H256};

#[test]
fn set_get_address() {
    new_test_ext(1).execute_with(|| {
        assert_ok!(Bridge::set_address(Origin::ROOT, vec![1, 2, 3, 4]));
        assert_eq!(<EmitterAddress>::get(), vec![1, 2, 3, 4])
    })
}

#[test]
fn asset_transfer_success() {
    new_test_ext(1).execute_with(|| {
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
    new_test_ext(1).execute_with(|| {
        let chain_id = vec![1];
        let to = vec![2];
        let bad_dest_id = vec![3];
        let token_id = vec![4];
        let metadata = vec![];

        assert_ok!(Bridge::whitelist_chain(Origin::ROOT, chain_id));
        assert_noop!(
            Bridge::receive_asset(Origin::ROOT, bad_dest_id, to, token_id, metadata),
            Error::<Test>::ChainNotWhitelisted
        );
    })
}

#[test]
fn transfer() {
    new_test_ext(1).execute_with(|| {
        // Check inital state
        assert_eq!(<EndowedAccount<Test>>::get(), ENDOWED_ID);
        assert_eq!(Balances::free_balance(&ENDOWED_ID), ENDOWED_BALANCE);
        // Transfer and check result
        assert_ok!(Bridge::transfer(Origin::ROOT, 2, 10));
        assert_eq!(Balances::free_balance(&ENDOWED_ID), ENDOWED_BALANCE - 10);
        assert_eq!(Balances::free_balance(2), 10);
    })
}


#[test]
fn add_remove_validator() {
    new_test_ext(1).execute_with(|| {
        // Already exists
        assert_noop!(Bridge::add_validator(Origin::ROOT, VALIDATOR_A), Error::<Test>::ValidatorAlreadyExists);

        // Errors if added twice
        assert_ok!(Bridge::add_validator(Origin::ROOT, 99));
        expect_event(RawEvent::ValidatorAdded(99));
        assert_noop!(Bridge::add_validator(Origin::ROOT, 99), Error::<Test>::ValidatorAlreadyExists);

        // Confirm removal
        assert_ok!(Bridge::remove_validator(Origin::ROOT, 99));
        expect_event(RawEvent::ValidatorRemoved(99));
        assert_noop!(Bridge::remove_validator(Origin::ROOT, 99), Error::<Test>::ValidatorInvalid);
    })
}

fn make_proposal(value: u64) -> mock::Call {
    mock::Call::System(frame_system::Call::remark(value.encode()))
}

#[test]
fn create_transfer_proposal() {
    new_test_ext(2).execute_with(|| {
        let prop_id: H256 = blake2_256("proposal".as_ref()).into();

        let call = make_proposal(10);

        assert_eq!(Bridge::validator_threshold(), 2);

        // Create proposal (& vote)
        assert_ok!(Bridge::create_proposal(Origin::signed(VALIDATOR_A), prop_id.clone(), Box::new(call.clone())));
        expect_event(RawEvent::VoteFor(prop_id, VALIDATOR_A));
        let prop = Bridge::proposals(prop_id).unwrap();
        let expected = TransferProposal {
            votes_for: vec![VALIDATOR_A],
            votes_against: vec![],
            call: Box::new(call.clone()),
        };
        assert_eq!(prop, expected);

        // Second validator votes against
        assert_ok!(Bridge::vote(Origin::signed(VALIDATOR_B), prop_id, false));
        expect_event(RawEvent::VoteAgainst(prop_id, VALIDATOR_B));
        let prop = Bridge::proposals(prop_id).unwrap();
        let expected = TransferProposal {
            votes_for: vec![VALIDATOR_A],
            votes_against: vec![VALIDATOR_B],
            call: Box::new(call.clone()),
        };
        assert_eq!(prop, expected);

        // Third validator votes in favour
        assert_ok!(Bridge::vote(Origin::signed(VALIDATOR_C), prop_id, true));
        let prop = Bridge::proposals(prop_id).unwrap();
        let expected = TransferProposal {
            votes_for: vec![VALIDATOR_A, VALIDATOR_C],
            votes_against: vec![VALIDATOR_B],
            call: Box::new(call.clone()),
        };
        assert_eq!(prop, expected);

        expect_event(RawEvent::ProposalSuceeded(prop_id));
    })
}