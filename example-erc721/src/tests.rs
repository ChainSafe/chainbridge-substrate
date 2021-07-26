// Copyright 2021 Centrifuge Foundation (centrifuge.io).
// This file is part of Centrifuge chain project.

// Centrifuge is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version (see http://www.gnu.org/licenses).

// Centrifuge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

//! # Example ERC721 pallet's unit test cases.

// ----------------------------------------------------------------------------
// Module imports and re-exports
// ----------------------------------------------------------------------------

use crate::{
    mock::*,
    *,
};

use frame_support::{
    assert_noop, 
    assert_ok
};

use sp_core::U256;


// ----------------------------------------------------------------------------
// Test cases
// ----------------------------------------------------------------------------

#[test]
fn mint_burn_tokens() {
    TestExternalitiesBuilder::default().build().execute_with(|| {
        let id_a: U256 = 1.into();
        let id_b: U256 = 2.into();
        let metadata_a: Vec<u8> = vec![1, 2, 3];
        let metadata_b: Vec<u8> = vec![4, 5, 6];

        assert_ok!(Erc721::mint(
            Origin::root(),
            USER_A,
            id_a,
            metadata_a.clone()
        ));
        assert_eq!(
            Erc721::get_tokens(id_a).unwrap(),
            Erc721Token {
                id: id_a,
                metadata: metadata_a.clone()
            }
        );
        assert_eq!(Erc721::get_token_count(), 1.into());
        assert_noop!(
            Erc721::mint(Origin::root(), USER_A, id_a, metadata_a.clone()),
            Error::<MockRuntime>::TokenAlreadyExists
        );

        assert_ok!(Erc721::mint(
            Origin::root(),
            USER_A,
            id_b,
            metadata_b.clone()
        ));
        assert_eq!(
            Erc721::get_tokens(id_b).unwrap(),
            Erc721Token {
                id: id_b,
                metadata: metadata_b.clone()
            }
        );
        assert_eq!(Erc721::get_token_count(), 2.into());
        assert_noop!(
            Erc721::mint(Origin::root(), USER_A, id_b, metadata_b.clone()),
            Error::<MockRuntime>::TokenAlreadyExists
        );

        assert_ok!(Erc721::burn(Origin::root(), id_a));
        assert_eq!(Erc721::get_token_count(), 1.into());
        assert!(!<Tokens<MockRuntime>>::contains_key(&id_a));
        assert!(!<TokenOwner<MockRuntime>>::contains_key(&id_a));

        assert_ok!(Erc721::burn(Origin::root(), id_b));
        assert_eq!(Erc721::get_token_count(), 0.into());
        assert!(!<Tokens<MockRuntime>>::contains_key(&id_b));
        assert!(!<TokenOwner<MockRuntime>>::contains_key(&id_b));
    })
}

#[test]
fn transfer_tokens() {
    TestExternalitiesBuilder::default().build().execute_with(|| {
        let id_a: U256 = 1.into();
        let id_b: U256 = 2.into();
        let metadata_a: Vec<u8> = vec![1, 2, 3];
        let metadata_b: Vec<u8> = vec![4, 5, 6];

        assert_ok!(Erc721::mint(
            Origin::root(),
            USER_A,
            id_a,
            metadata_a.clone()
        ));
        assert_ok!(Erc721::mint(
            Origin::root(),
            USER_A,
            id_b,
            metadata_b.clone()
        ));

        assert_ok!(Erc721::transfer(Origin::signed(USER_A), USER_B, id_a));
        assert_eq!(Erc721::get_owner_of(id_a).unwrap(), USER_B);

        assert_ok!(Erc721::transfer(Origin::signed(USER_A), USER_C, id_b));
        assert_eq!(Erc721::get_owner_of(id_b).unwrap(), USER_C);

        assert_ok!(Erc721::transfer(Origin::signed(USER_B), USER_A, id_a));
        assert_eq!(Erc721::get_owner_of(id_a).unwrap(), USER_A);

        assert_ok!(Erc721::transfer(Origin::signed(USER_C), USER_A, id_b));
        assert_eq!(Erc721::get_owner_of(id_b).unwrap(), USER_A);
    })
}
