#![cfg(test)]

use super::mock::{
    assert_events, new_test_ext, Balances, Bridge, Call, Event, Origin, Test, ENDOWED_BALANCE,
    RELAYER_A, RELAYER_B, RELAYER_C,
};
use super::*;
use frame_support::{assert_noop, assert_ok};

const TEST_THRESHOLD: u32 = 2;
const TEST_CHAIN_ID: u32 = 5;

#[test]
fn initialize_chain() {
    new_test_ext().execute_with(|| {
        assert_ok!(Bridge::add_relayer(Origin::ROOT, RELAYER_A));

        let prop_id = 1;
        let proposal = make_proposal(vec![10]);
        let dest_id = 2;
        let to = vec![2];
        let token_id = vec![3];
        let metadata = vec![];

        assert_noop!(
            Bridge::acknowledge_proposal(
                Origin::signed(RELAYER_A),
                prop_id,
                Box::new(proposal.clone())
            ),
            Error::<Test>::NotInitialized
        );

        assert_noop!(
            Bridge::receive_asset(
                Origin::ROOT,
                dest_id.clone(),
                to.clone(),
                token_id.clone(),
                metadata.clone()
            ),
            Error::<Test>::NotInitialized
        );

        assert_ok!(Bridge::initialize(
            Origin::ROOT,
            TEST_THRESHOLD,
            TEST_CHAIN_ID
        ));
    })
}

#[test]
fn whitelist_chain() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Bridge::whitelist_chain(Origin::ROOT, 0),
            Error::<Test>::NotInitialized
        );

        assert_ok!(Bridge::initialize(
            Origin::ROOT,
            TEST_THRESHOLD,
            TEST_CHAIN_ID
        ));

        assert_ok!(Bridge::whitelist_chain(Origin::ROOT, 0));
        assert_noop!(
            Bridge::whitelist_chain(Origin::ROOT, Bridge::chain_id()),
            Error::<Test>::InvalidChainId
        );

        assert_events(vec![Event::bridge(RawEvent::ChainWhitelisted(0))]);
    })
}

#[test]
fn set_get_threshold() {
    new_test_ext().execute_with(|| {
        assert_eq!(<RelayerThreshold>::get(), 0);

        assert_ok!(Bridge::initialize(
            Origin::ROOT,
            TEST_THRESHOLD,
            TEST_CHAIN_ID
        ));
        assert_eq!(<RelayerThreshold>::get(), TEST_THRESHOLD);

        assert_ok!(Bridge::set_threshold(Origin::ROOT, 5));
        assert_eq!(<RelayerThreshold>::get(), 5);

        assert_events(vec![Event::bridge(RawEvent::RelayerThresholdChanged(5))]);
    })
}

#[test]
fn asset_transfer_success() {
    new_test_ext().execute_with(|| {
        let dest_id = 2;
        let to = vec![2];
        let token_id = vec![3];
        let metadata = vec![];

        assert_ok!(Bridge::initialize(
            Origin::ROOT,
            TEST_THRESHOLD,
            TEST_CHAIN_ID
        ));

        assert_ok!(Bridge::whitelist_chain(Origin::ROOT, dest_id.clone()));
        assert_ok!(Bridge::receive_asset(
            Origin::ROOT,
            dest_id.clone(),
            to.clone(),
            token_id.clone(),
            metadata.clone()
        ));
        assert_events(vec![
            Event::bridge(RawEvent::ChainWhitelisted(dest_id.clone())),
            Event::bridge(RawEvent::AssetTransfer(
                dest_id.clone(),
                0,
                to,
                token_id,
                metadata,
            )),
        ]);
    })
}

#[test]
fn asset_transfer_invalid_chain() {
    new_test_ext().execute_with(|| {
        let chain_id = 2;
        let to = vec![2];
        let bad_dest_id = 3;
        let token_id = vec![4];
        let metadata = vec![];

        assert_ok!(Bridge::initialize(
            Origin::ROOT,
            TEST_THRESHOLD,
            TEST_CHAIN_ID
        ));

        assert_ok!(Bridge::whitelist_chain(Origin::ROOT, chain_id.clone()));
        assert_noop!(
            Bridge::receive_asset(
                Origin::ROOT,
                bad_dest_id,
                to.clone(),
                token_id.clone(),
                metadata.clone()
            ),
            Error::<Test>::ChainNotWhitelisted
        );
        assert_events(vec![Event::bridge(RawEvent::ChainWhitelisted(
            chain_id.clone(),
        ))]);
    })
}

#[test]
fn add_remove_relayer() {
    new_test_ext().execute_with(|| {
        assert_ok!(Bridge::initialize(
            Origin::ROOT,
            TEST_THRESHOLD,
            TEST_CHAIN_ID
        ));
        assert_eq!(Bridge::relayer_count(), 0);

        assert_ok!(Bridge::add_relayer(Origin::ROOT, RELAYER_A));
        assert_ok!(Bridge::add_relayer(Origin::ROOT, RELAYER_B));
        assert_ok!(Bridge::add_relayer(Origin::ROOT, RELAYER_C));
        assert_eq!(Bridge::relayer_count(), 3);

        // Already exists
        assert_noop!(
            Bridge::add_relayer(Origin::ROOT, RELAYER_A),
            Error::<Test>::RelayerAlreadyExists
        );

        // Confirm removal
        assert_ok!(Bridge::remove_relayer(Origin::ROOT, RELAYER_B));
        assert_eq!(Bridge::relayer_count(), 2);
        assert_noop!(
            Bridge::remove_relayer(Origin::ROOT, RELAYER_B),
            Error::<Test>::RelayerInvalid
        );
        assert_eq!(Bridge::relayer_count(), 2);

        assert_events(vec![
            Event::bridge(RawEvent::RelayerAdded(RELAYER_A)),
            Event::bridge(RawEvent::RelayerAdded(RELAYER_B)),
            Event::bridge(RawEvent::RelayerAdded(RELAYER_C)),
            Event::bridge(RawEvent::RelayerRemoved(RELAYER_B)),
        ]);
    })
}

fn make_proposal(r: Vec<u8>) -> mock::Call {
    Call::System(system::Call::remark(r))
}

#[test]
fn create_sucessful_proposal() {
    new_test_ext().execute_with(|| {
        let prop_id = 1;
        let proposal = make_proposal(vec![10]);

        assert_ok!(Bridge::initialize(
            Origin::ROOT,
            TEST_THRESHOLD,
            TEST_CHAIN_ID
        ));
        assert_ok!(Bridge::add_relayer(Origin::ROOT, RELAYER_A));
        assert_ok!(Bridge::add_relayer(Origin::ROOT, RELAYER_B));
        assert_ok!(Bridge::add_relayer(Origin::ROOT, RELAYER_C));
        assert_eq!(Bridge::relayer_threshold(), 2);

        // Create proposal (& vote)
        assert_ok!(Bridge::acknowledge_proposal(
            Origin::signed(RELAYER_A),
            prop_id,
            Box::new(proposal.clone())
        ));
        let prop = Bridge::votes((prop_id.clone(), proposal.clone())).unwrap();
        let expected = ProposalVotes {
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
        let expected = ProposalVotes {
            votes_for: vec![RELAYER_A],
            votes_against: vec![RELAYER_B],
        };
        assert_eq!(prop, expected);

        // Third relayer votes in favour
        assert_ok!(Bridge::acknowledge_proposal(
            Origin::signed(RELAYER_C),
            prop_id,
            Box::new(proposal.clone())
        ));
        let prop = Bridge::votes((prop_id.clone(), proposal.clone())).unwrap();
        let expected = ProposalVotes {
            votes_for: vec![RELAYER_A, RELAYER_C],
            votes_against: vec![RELAYER_B],
        };
        assert_eq!(prop, expected);

        assert_events(vec![
            Event::bridge(RawEvent::VoteFor(prop_id, RELAYER_A)),
            Event::bridge(RawEvent::VoteAgainst(prop_id, RELAYER_B)),
            Event::bridge(RawEvent::VoteFor(prop_id, RELAYER_C)),
            Event::bridge(RawEvent::ProposalApproved(prop_id)),
            Event::bridge(RawEvent::ProposalSucceeded(prop_id)),
        ]);
    })
}

#[test]
fn create_unsucessful_proposal() {
    new_test_ext().execute_with(|| {
        let prop_id = 1;
        let proposal = make_proposal(vec![11]);

        assert_ok!(Bridge::initialize(
            Origin::ROOT,
            TEST_THRESHOLD,
            TEST_CHAIN_ID
        ));
        assert_ok!(Bridge::add_relayer(Origin::ROOT, RELAYER_A));
        assert_ok!(Bridge::add_relayer(Origin::ROOT, RELAYER_B));
        assert_ok!(Bridge::add_relayer(Origin::ROOT, RELAYER_C));

        // Create proposal (& vote)
        assert_ok!(Bridge::acknowledge_proposal(
            Origin::signed(RELAYER_A),
            prop_id.clone(),
            Box::new(proposal.clone())
        ));
        let prop = Bridge::votes((prop_id.clone(), proposal.clone())).unwrap();
        let expected = ProposalVotes {
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
        let expected = ProposalVotes {
            votes_for: vec![RELAYER_A],
            votes_against: vec![RELAYER_B],
        };
        assert_eq!(prop, expected);

        // Third relayer votes against
        assert_ok!(Bridge::reject(
            Origin::signed(RELAYER_C),
            prop_id,
            Box::new(proposal.clone())
        ));
        let prop = Bridge::votes((prop_id.clone(), proposal.clone())).unwrap();
        let expected = ProposalVotes {
            votes_for: vec![RELAYER_A],
            votes_against: vec![RELAYER_B, RELAYER_C],
        };
        assert_eq!(prop, expected);

        assert_eq!(Balances::free_balance(RELAYER_B), 0);
        assert_eq!(
            Balances::free_balance(Bridge::account_id()),
            ENDOWED_BALANCE
        );

        assert_events(vec![
            Event::bridge(RawEvent::VoteFor(prop_id, RELAYER_A)),
            Event::bridge(RawEvent::VoteAgainst(prop_id, RELAYER_B)),
            Event::bridge(RawEvent::VoteAgainst(prop_id, RELAYER_C)),
            Event::bridge(RawEvent::ProposalRejected(prop_id)),
        ]);
    })
}
