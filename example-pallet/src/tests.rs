#![cfg(test)]

use super::mock::{
    assert_events, balances, event_exists, expect_event, new_test_ext, Balances, Bridge, Call,
    Event, Example, Origin, ENDOWED_BALANCE, RELAYER_A, RELAYER_B, RELAYER_C,
};
use super::*;
use frame_support::dispatch::DispatchError;
use frame_support::{assert_noop, assert_ok};

use codec::Encode;
use sp_core::{blake2_256, H256};

fn make_remark_proposal(hash: H256) -> Call {
    Call::Example(crate::Call::remark(hash))
}

fn make_transfer_proposal(to: u64, amount: u32) -> Call {
    Call::Example(crate::Call::transfer(to, amount))
}

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
            vec![1],
            hash.as_ref().to_vec(),
        ));
    })
}

#[test]
fn execute_remark() {
    new_test_ext(2).execute_with(|| {
        let hash: H256 = "ABC".using_encoded(blake2_256).into();
        let proposal = make_remark_proposal(hash.clone());
        let prop_id = 1;

        assert_ok!(Bridge::create_proposal(
            Origin::signed(RELAYER_A),
            prop_id,
            Box::new(proposal.clone())
        ));
        assert_ok!(Bridge::approve(
            Origin::signed(RELAYER_B),
            prop_id,
            Box::new(proposal.clone())
        ));

        event_exists(RawEvent::Remark(hash));
    })
}

#[test]
fn execute_remark_bad_origin() {
    new_test_ext(2).execute_with(|| {
        let hash: H256 = "ABC".using_encoded(blake2_256).into();

        assert_ok!(Example::remark(Origin::signed(Bridge::account_id()), hash));
        // Don't allow any signed origin except from bridge addr
        assert_noop!(
            Example::remark(Origin::signed(RELAYER_A), hash),
            DispatchError::BadOrigin
        );
        // Don't allow root calls
        assert_noop!(
            Example::remark(Origin::ROOT, hash),
            DispatchError::BadOrigin
        );
    })
}

#[test]
fn transfer() {
    new_test_ext(1).execute_with(|| {
        // Check inital state
        let bridge_id: u64 = Bridge::account_id();
        assert_eq!(Balances::free_balance(&bridge_id), ENDOWED_BALANCE);
        // Transfer and check result
        assert_ok!(Example::transfer(
            Origin::signed(Bridge::account_id()),
            RELAYER_A,
            10
        ));
        assert_eq!(Balances::free_balance(&bridge_id), ENDOWED_BALANCE - 10);
        assert_eq!(Balances::free_balance(RELAYER_A), 10);

        assert_events(vec![Event::balances(balances::RawEvent::Transfer(
            Bridge::account_id(),
            RELAYER_A,
            10,
        ))]);
    })
}

#[test]
fn create_sucessful_transfer_proposal() {
    new_test_ext(2).execute_with(|| {
        let prop_id = 1;

        let proposal = make_transfer_proposal(RELAYER_A, 10);

        assert_eq!(Bridge::relayer_threshold(), 2);

        // Create proposal (& vote)
        assert_ok!(Bridge::create_proposal(
            Origin::signed(RELAYER_A),
            prop_id,
            Box::new(proposal.clone())
        ));
        let prop = Bridge::votes((prop_id.clone(), proposal.clone())).unwrap();
        let expected = bridge::ProposalVotes {
            votes_for: vec![RELAYER_A],
            votes_against: vec![],
        };
        assert_eq!(prop, expected);

        // Second relayer votes against
        assert_ok!(Bridge::reject(
            Origin::signed(RELAYER_B),
            prop_id,
            Box::new(proposal.clone())
        ));
        let prop = Bridge::votes((prop_id.clone(), proposal.clone())).unwrap();
        let expected = bridge::ProposalVotes {
            votes_for: vec![RELAYER_A],
            votes_against: vec![RELAYER_B],
        };
        assert_eq!(prop, expected);

        // Third relayer votes in favour
        assert_ok!(Bridge::approve(
            Origin::signed(RELAYER_C),
            prop_id,
            Box::new(proposal.clone())
        ));
        let prop = Bridge::votes((prop_id.clone(), proposal.clone())).unwrap();
        let expected = bridge::ProposalVotes {
            votes_for: vec![RELAYER_A, RELAYER_C],
            votes_against: vec![RELAYER_B],
        };
        assert_eq!(prop, expected);

        assert_eq!(Balances::free_balance(RELAYER_A), 10);
        assert_eq!(
            Balances::free_balance(Bridge::account_id()),
            ENDOWED_BALANCE - 10
        );

        assert_events(vec![
            Event::bridge(bridge::RawEvent::VoteFor(prop_id, RELAYER_A)),
            Event::bridge(bridge::RawEvent::VoteAgainst(prop_id, RELAYER_B)),
            Event::bridge(bridge::RawEvent::VoteFor(prop_id, RELAYER_C)),
            Event::bridge(bridge::RawEvent::ProposalApproved(prop_id)),
            Event::system(system::RawEvent::NewAccount(RELAYER_A)),
            Event::balances(balances::RawEvent::Endowed(RELAYER_A, 10)),
            Event::balances(balances::RawEvent::Transfer(
                Bridge::account_id(),
                RELAYER_A,
                10,
            )),
            Event::bridge(bridge::RawEvent::ProposalSucceeded(prop_id)),
        ]);
    })
}
