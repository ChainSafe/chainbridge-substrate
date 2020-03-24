#![cfg(test)]

use super::mock::{
    new_test_ext, Balances, Bridge, Origin, Test, ENDOWED_BALANCE, RELAYER_A, RELAYER_B, RELAYER_C,
};
use super::*;
use frame_support::{assert_noop, assert_ok};

use sp_core::{blake2_256, H256};

type System = frame_system::Module<Test>;

#[test]
fn genesis_relayers_generated() {
    new_test_ext(2).execute_with(|| {
        System::set_block_number(1);
        assert_eq!(<Relayers<Test>>::get(RELAYER_A), true);
        assert_eq!(<Relayers<Test>>::get(RELAYER_B), true);
        assert_eq!(<Relayers<Test>>::get(RELAYER_C), true);
        assert_eq!(<RelayerCount>::get(), 3);
        assert_eq!(<RelayerThreshold>::get(), 2);
    });
}

#[test]
fn set_get_id() {
    new_test_ext(1).execute_with(|| {
        assert_ok!(Bridge::set_id(Origin::ROOT, 99));
        assert_eq!(<ChainId>::get(), 99)
    })
}

#[test]
fn set_get_threshold() {
    new_test_ext(1).execute_with(|| {
        assert_eq!(<RelayerThreshold>::get(), 1);

        assert_ok!(Bridge::set_threshold(Origin::ROOT, 5));
        assert_eq!(<RelayerThreshold>::get(), 5)
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
        let bridge_id: u64 = Bridge::account_id();
        assert_eq!(Balances::free_balance(&bridge_id), ENDOWED_BALANCE);
        // Transfer and check result
        assert_ok!(Bridge::transfer(
            Origin::signed(Bridge::account_id()),
            2,
            10
        ));
        assert_eq!(Balances::free_balance(&bridge_id), ENDOWED_BALANCE - 10);
        assert_eq!(Balances::free_balance(2), 10);
    })
}

#[test]
fn add_remove_relayer() {
    new_test_ext(1).execute_with(|| {
        assert_eq!(Bridge::relayer_count(), 3);
        // Already exists
        assert_noop!(
            Bridge::add_relayer(Origin::ROOT, RELAYER_A),
            Error::<Test>::RelayerAlreadyExists
        );

        // Errors if added twice
        assert_ok!(Bridge::add_relayer(Origin::ROOT, 99));
        assert_eq!(Bridge::relayer_count(), 4);
        assert_noop!(
            Bridge::add_relayer(Origin::ROOT, 99),
            Error::<Test>::RelayerAlreadyExists
        );

        // Confirm removal
        assert_ok!(Bridge::remove_relayer(Origin::ROOT, 99));
        assert_eq!(Bridge::relayer_count(), 3);
        assert_noop!(
            Bridge::remove_relayer(Origin::ROOT, 99),
            Error::<Test>::RelayerInvalid
        );
    })
}

fn make_proposal(to: u64, amount: u32) -> mock::Call {
    mock::Call::Bridge(crate::Call::transfer(to, amount))
}

#[test]
fn create_sucessful_transfer_proposal() {
    new_test_ext(2).execute_with(|| {
        let prop_id = 1;

        let proposal = make_proposal(RELAYER_A, 10);

        assert_eq!(Bridge::relayer_threshold(), 2);

        // Create proposal (& vote)
        assert_ok!(Bridge::create_proposal(
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
        assert_ok!(Bridge::approve(
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

        assert_eq!(Balances::free_balance(RELAYER_A), 10);
        assert_eq!(
            Balances::free_balance(Bridge::account_id()),
            ENDOWED_BALANCE - 10
        );
    })
}

#[test]
fn create_unsucessful_transfer_proposal() {
    new_test_ext(2).execute_with(|| {
        let prop_id = 1;

        let proposal = make_proposal(RELAYER_B, 10);

        assert_eq!(Bridge::relayer_threshold(), 2);

        // Create proposal (& vote)
        assert_ok!(Bridge::create_proposal(
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
    })
}
