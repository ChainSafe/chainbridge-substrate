#![deny(warnings)]

use codec::{
    Decode,
    Encode,
};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_std::prelude::*;

pub type ChainId = u8;
pub type DepositNonce = u64;
pub type ResourceId = [u8; 32];

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum ProposalStatus {
    Initiated,
    Approved,
    Rejected,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ProposalVotes<AccountId, BlockNumber> {
    pub votes_for: Vec<AccountId>,
    pub votes_against: Vec<AccountId>,
    pub status: ProposalStatus,
    pub expiry: BlockNumber,
}

impl<AccountId, BlockNumber> Default for ProposalVotes<AccountId, BlockNumber>
where
    BlockNumber: Default,
{
    fn default() -> Self {
        Self {
            votes_for: vec![],
            votes_against: vec![],
            status: ProposalStatus::Initiated,
            expiry: BlockNumber::default(),
        }
    }
}

impl<AccountId, BlockNumber> ProposalVotes<AccountId, BlockNumber>
where
    AccountId: PartialEq,
    BlockNumber: PartialOrd,
{
    /// Attempts to mark the proposal as approve or rejected.
    /// Returns true if the status changes from active.
    /// TODO: use saturating_add
    pub(crate) fn try_to_complete(
        &mut self,
        threshold: u32,
        total: u32,
    ) -> ProposalStatus {
        if self.votes_for.len() >= threshold as usize {
            self.status = ProposalStatus::Approved;
            ProposalStatus::Approved
        } else if total >= threshold
            && self.votes_against.len() as u32 + threshold > total
        {
            self.status = ProposalStatus::Rejected;
            ProposalStatus::Rejected
        } else {
            ProposalStatus::Initiated
        }
    }

    /// Returns true if the proposal has been rejected or approved, otherwise false.
    pub(crate) fn is_complete(&self) -> bool {
        self.status != ProposalStatus::Initiated
    }

    /// Returns true if the `who` has voted for or against the proposal
    pub(crate) fn has_voted(&self, who: &AccountId) -> bool {
        self.votes_for.contains(&who) || self.votes_against.contains(&who)
    }

    /// Returns true if the expiry time has been reached
    pub(crate) fn is_expired(&self, now: BlockNumber) -> bool {
        self.expiry <= now
    }
}
