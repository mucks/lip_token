use ink_env::AccountId;
use ink_lang as ink;
use scale::{Decode, Encode};

/// A token ID.
pub type TokenId = u32;

#[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    NotOwner,
    NotApproved,
    TokenExists,
    TokenNotFound,
    CannotInsert,
    CannotFetchValue,
    NotAllowed,
}

#[ink::trait_definition]
pub trait BaseErc721 {
    /// Returns the balance of the owner.
    ///
    /// This represents the amount of unique tokens the owner has.
    #[ink(message)]
    fn balance_of(&self, owner: AccountId) -> u32;

    /// Returns the owner of the token.
    #[ink(message)]
    fn owner_of(&self, id: TokenId) -> Option<AccountId>;

    /// Returns the approved account ID for this token if any.
    #[ink(message)]
    fn get_approved(&self, id: TokenId) -> Option<AccountId>;

    /// Returns `true` if the operator is approved by the owner.
    #[ink(message)]
    fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool;

    /// Approves or disapproves the operator for all tokens of the caller.
    #[ink(message)]
    fn set_approval_for_all(&mut self, to: AccountId, approved: bool) -> Result<(), Error>;

    /// Approves the account to transfer the specified token on behalf of the caller.
    #[ink(message)]
    fn approve(&mut self, to: AccountId, id: TokenId) -> Result<(), Error>;

    /// Transfers the token from the caller to the given destination.
    #[ink(message)]
    fn transfer(&mut self, destination: AccountId, id: TokenId) -> Result<(), Error>;

    /// Transfer approved or owned token.
    #[ink(message)]
    fn transfer_from(&mut self, from: AccountId, to: AccountId, id: TokenId) -> Result<(), Error>;

    /// Creates a new token.
    #[ink(message)]
    fn mint(&mut self, id: TokenId) -> Result<(), Error>;

    /// Deletes an existing token. Only the owner can burn the token.
    #[ink(message)]
    fn burn(&mut self, id: TokenId) -> Result<(), Error>;
}
