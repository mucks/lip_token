#![cfg_attr(not(feature = "std"), no_std)]

mod base_erc721;

use ink_lang as ink;

#[ink::contract]
mod lip_token {
    use crate::base_erc721::{BaseErc721, Error};
    use ink_prelude::string::String;
    use ink_prelude::vec::Vec;
    //INFO: you can derive these (no need for other crate)
    use ink_prelude::format;
    use ink_storage::traits::{PackedLayout, SpreadAllocate, SpreadLayout};
    use ink_storage::Mapping;

    /// A token ID.
    pub type TokenId = u32;

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    #[derive(Default, SpreadAllocate)]
    pub struct LipToken {
        /// Counter of Tokens
        counter: u32,
        owner: AccountId,
        fee: u128,
        lips: Vec<Lip>,
        /// Mapping from token to owner.
        token_owner: Mapping<TokenId, AccountId>,
        /// Mapping from token to approvals users.
        token_approvals: Mapping<TokenId, AccountId>,
        /// Mapping from owner to number of owned token.
        owned_tokens_count: Mapping<AccountId, u32>,
        /// Mapping from owner to operator approvals.
        operator_approvals: Mapping<(AccountId, AccountId), ()>,
    }

    #[derive(
        Default,
        Debug,
        Clone,
        scale::Encode,
        scale::Decode,
        SpreadLayout,
        PackedLayout,
        SpreadAllocate,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Lip {
        name: ink_prelude::string::String,
        id: u32,
        dna: Hash,
        level: u8,
        rarity: u8,
    }

    //TODO: find out what this does
    impl ink_storage::traits::PackedAllocate for Lip {
        fn allocate_packed(&mut self, key: &ink_primitives::Key) {}
    }

    /// Event emitted when a token transfer occurs.
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        id: TokenId,
    }

    /// Event emitted when a token approve occurs.
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        id: TokenId,
    }

    /// Event emitted when an operator is enabled or disabled for an owner.
    /// The operator can manage all NFTs of the owner.
    #[ink(event)]
    pub struct ApprovalForAll {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        approved: bool,
    }

    #[ink(event)]
    pub struct NewLip {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        id: u32,
        #[ink(topic)]
        dna: Hash,
    }

    impl LipToken {
        #[ink(constructor)]
        pub fn new() -> Self {
            ink_lang::utils::initialize_contract(|contract: &mut Self| {
                let caller = Self::env().caller();
                contract.owner = caller;
                contract.counter = 1;
                contract.fee = 1;
            })
        }

        fn only_owner(&self) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller == self.owner {
                Ok(())
            } else {
                Err(Error::NotOwner)
            }
        }

        // Helpers

        fn gen_random_dna(&self) -> Result<Hash, Error> {
            let timestamp = format!("{}", self.env().block_timestamp());
            let (hash, _block) = self.env().random(timestamp.as_bytes());
            Ok(hash)
        }

        fn gen_random_u8(&self) -> u8 {
            let timestamp = format!("{}", self.env().block_timestamp());
            let (hash, _block) = self.env().random(timestamp.as_bytes());
            let n: u8 = hash.as_ref()[0];
            n
        }

        // Update the fee for minting lips
        #[ink(message)]
        pub fn update_fee(&mut self, fee: u128) -> Result<(), Error> {
            self.only_owner()?;
            self.fee = fee;

            Ok(())
        }

        // /Helpers

        // Creation
        fn create_lip(&mut self, name: String) -> Result<(), Error> {
            let caller = self.env().caller();

            //let random_rarity = self.create_random_num(U256::new(100))?;
            let random_dna = self.gen_random_dna()?;
            let random_rarity = self.gen_random_u8();

            let lip = Lip {
                name,
                id: self.counter,
                dna: random_dna,
                level: 1,
                rarity: random_rarity,
            };

            let new_lip_event = NewLip {
                id: lip.id,
                dna: lip.dna,
                owner: caller,
            };
            self.lips.push(lip);

            self.env().emit_event(new_lip_event);
            self.safe_mint(&caller, self.counter)?;
            self.counter += 1;
            Ok(())
        }

        ///Create Random Lip with random DNA and random rarity
        #[ink(message, payable)]
        pub fn create_random_lip(&mut self, name: String) -> Result<(), Error> {
            assert_eq!(
                self.env().transferred_value(),
                self.fee,
                "The Fee is not correct"
            );
            self.create_lip(name)
        }

        // /Creation

        //Getters

        /// Get Lips
        #[ink(message)]
        pub fn get_lips(&self) -> Vec<Lip> {
            return self.lips.clone();
        }

        // /Getters

        #[ink(message)]
        pub fn mint_my_nft(&mut self, to: AccountId) -> Result<(), Error> {
            self.only_owner()?;
            self.safe_mint(&to, self.counter)?;
            self.counter += 1;
            Ok(())
        }

        fn safe_mint(&mut self, to: &AccountId, counter: u32) -> Result<(), Error> {
            let caller = self.env().caller();
            self.add_token_to(to, counter)?;
            self.env().emit_event(Transfer {
                from: Some(AccountId::from([0x0; 32])),
                to: Some(caller),
                id: counter,
            });
            Ok(())
        }

        /// Transfers token `id` `from` the sender to the `to` `AccountId`.
        fn transfer_token_from(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            id: TokenId,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            if !self.exists(id) {
                return Err(Error::TokenNotFound);
            };
            if !self.approved_or_owner(Some(caller), id) {
                return Err(Error::NotApproved);
            };
            self.clear_approval(id);
            self.remove_token_from(from, id)?;
            self.add_token_to(to, id)?;
            self.env().emit_event(Transfer {
                from: Some(*from),
                to: Some(*to),
                id,
            });
            Ok(())
        }

        /// Removes token `id` from the owner.
        fn remove_token_from(&mut self, from: &AccountId, id: TokenId) -> Result<(), Error> {
            let Self {
                token_owner,
                owned_tokens_count,
                ..
            } = self;

            if !token_owner.contains(&id) {
                return Err(Error::TokenNotFound);
            }

            let count = owned_tokens_count
                .get(&from)
                .map(|c| c - 1)
                .ok_or(Error::CannotFetchValue)?;
            owned_tokens_count.insert(&from, &count);
            token_owner.remove(&id);

            Ok(())
        }

        /// Adds the token `id` to the `to` AccountID.
        fn add_token_to(&mut self, to: &AccountId, id: TokenId) -> Result<(), Error> {
            let Self {
                token_owner,
                owned_tokens_count,
                ..
            } = self;

            if token_owner.contains(&id) {
                return Err(Error::TokenExists);
            }

            if *to == AccountId::from([0x0; 32]) {
                return Err(Error::NotAllowed);
            };

            let count = owned_tokens_count.get(to).map(|c| c + 1).unwrap_or(1);

            owned_tokens_count.insert(to, &count);
            token_owner.insert(&id, to);

            Ok(())
        }

        /// Approves or disapproves the operator to transfer all tokens of the caller.
        fn approve_for_all(&mut self, to: AccountId, approved: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            if to == caller {
                return Err(Error::NotAllowed);
            }
            self.env().emit_event(ApprovalForAll {
                owner: caller,
                operator: to,
                approved,
            });

            if approved {
                self.operator_approvals.insert((&caller, &to), &());
            } else {
                self.operator_approvals.remove((&caller, &to));
            }

            Ok(())
        }

        /// Approve the passed `AccountId` to transfer the specified token on behalf of the message's sender.
        fn approve_for(&mut self, to: &AccountId, id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            let owner = self.owner_of(id);
            if !(owner == Some(caller)
                || self.approved_for_all(owner.expect("Error with AccountId"), caller))
            {
                return Err(Error::NotAllowed);
            };

            if *to == AccountId::from([0x0; 32]) {
                return Err(Error::NotAllowed);
            };

            if self.token_approvals.contains(&id) {
                return Err(Error::CannotInsert);
            } else {
                self.token_approvals.insert(&id, to);
            }

            self.env().emit_event(Approval {
                from: caller,
                to: *to,
                id,
            });

            Ok(())
        }

        /// Removes existing approval from token `id`.
        fn clear_approval(&mut self, id: TokenId) {
            self.token_approvals.remove(&id);
        }

        // Returns the total number of tokens from an account.
        fn balance_of_or_zero(&self, of: &AccountId) -> u32 {
            self.owned_tokens_count.get(of).unwrap_or(0)
        }

        /// Gets an operator on other Account's behalf.
        fn approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.operator_approvals.contains((&owner, &operator))
        }

        /// Returns true if the `AccountId` `from` is the owner of token `id`
        /// or it has been approved on behalf of the token `id` owner.
        fn approved_or_owner(&self, from: Option<AccountId>, id: TokenId) -> bool {
            let owner = self.owner_of(id);
            from != Some(AccountId::from([0x0; 32]))
                && (from == owner
                    || from == self.token_approvals.get(&id)
                    || self.approved_for_all(
                        owner.expect("Error with AccountId"),
                        from.expect("Error with AccountId"),
                    ))
        }

        /// Returns true if token `id` exists or false if it does not.
        fn exists(&self, id: TokenId) -> bool {
            self.token_owner.contains(&id)
        }
    }

    impl BaseErc721 for LipToken {
        /// Returns the balance of the owner.
        ///
        /// This represents the amount of unique tokens the owner has.
        #[ink(message)]
        fn balance_of(&self, owner: AccountId) -> u32 {
            self.balance_of_or_zero(&owner)
        }

        /// Returns the owner of the token.
        #[ink(message)]
        fn owner_of(&self, id: TokenId) -> Option<AccountId> {
            self.token_owner.get(&id)
        }

        /// Returns the approved account ID for this token if any.
        #[ink(message)]
        fn get_approved(&self, id: TokenId) -> Option<AccountId> {
            self.token_approvals.get(&id)
        }

        /// Returns `true` if the operator is approved by the owner.
        #[ink(message)]
        fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.approved_for_all(owner, operator)
        }

        /// Approves or disapproves the operator for all tokens of the caller.
        #[ink(message)]
        fn set_approval_for_all(&mut self, to: AccountId, approved: bool) -> Result<(), Error> {
            self.approve_for_all(to, approved)?;
            Ok(())
        }

        /// Approves the account to transfer the specified token on behalf of the caller.
        #[ink(message)]
        fn approve(&mut self, to: AccountId, id: TokenId) -> Result<(), Error> {
            self.approve_for(&to, id)?;
            Ok(())
        }

        /// Transfers the token from the caller to the given destination.
        #[ink(message)]
        fn transfer(&mut self, destination: AccountId, id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            self.transfer_token_from(&caller, &destination, id)?;
            Ok(())
        }

        /// Transfer approved or owned token.
        #[ink(message)]
        fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            id: TokenId,
        ) -> Result<(), Error> {
            self.transfer_token_from(&from, &to, id)?;
            Ok(())
        }

        /// Creates a new token.
        #[ink(message)]
        fn mint(&mut self, id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            self.add_token_to(&caller, id)?;
            self.env().emit_event(Transfer {
                from: Some(AccountId::from([0x0; 32])),
                to: Some(caller),
                id,
            });
            Ok(())
        }

        /// Deletes an existing token. Only the owner can burn the token.
        #[ink(message)]
        fn burn(&mut self, id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            let Self {
                token_owner,
                owned_tokens_count,
                ..
            } = self;

            let owner = token_owner.get(&id).ok_or(Error::TokenNotFound)?;
            if owner != caller {
                return Err(Error::NotOwner);
            };

            let count = owned_tokens_count
                .get(&caller)
                .map(|c| c - 1)
                .ok_or(Error::CannotFetchValue)?;
            owned_tokens_count.insert(&caller, &count);
            token_owner.remove(&id);

            self.env().emit_event(Transfer {
                from: Some(caller),
                to: Some(AccountId::from([0x0; 32])),
                id,
            });

            Ok(())
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// Imports `ink_lang` so we can use `#[ink::test]`.
        use ink_lang as ink;
        #[ink::test]
        fn mint_works() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = LipToken::new();
            // Token 1 does not exists.
            assert_eq!(erc721.owner_of(1), None);
            // Alice does not owns tokens.
            assert_eq!(erc721.balance_of(accounts.alice), 0);
            // Create token Id 1.
            assert_eq!(erc721.mint(1), Ok(()));
            // Alice owns 1 token.
            assert_eq!(erc721.balance_of(accounts.alice), 1);
        }

        #[ink::test]
        fn mint_existing_should_fail() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = LipToken::new();
            // Create token Id 1.
            assert_eq!(erc721.mint(1), Ok(()));
            // The first Transfer event takes place
            assert_eq!(1, ink_env::test::recorded_events().count());
            // Alice owns 1 token.
            assert_eq!(erc721.balance_of(accounts.alice), 1);
            // Alice owns token Id 1.
            assert_eq!(erc721.owner_of(1), Some(accounts.alice));
            // Cannot create  token Id if it exists.
            // Bob cannot own token Id 1.
            assert_eq!(erc721.mint(1), Err(Error::TokenExists));
        }

        #[ink::test]
        fn transfer_works() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = LipToken::new();
            // Create token Id 1 for Alice
            assert_eq!(erc721.mint(1), Ok(()));
            // Alice owns token 1
            assert_eq!(erc721.balance_of(accounts.alice), 1);
            // Bob does not owns any token
            assert_eq!(erc721.balance_of(accounts.bob), 0);
            // The first Transfer event takes place
            assert_eq!(1, ink_env::test::recorded_events().count());
            // Alice transfers token 1 to Bob
            assert_eq!(erc721.transfer(accounts.bob, 1), Ok(()));
            // The second Transfer event takes place
            assert_eq!(2, ink_env::test::recorded_events().count());
            // Bob owns token 1
            assert_eq!(erc721.balance_of(accounts.bob), 1);
        }

        #[ink::test]
        fn invalid_transfer_should_fail() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = LipToken::new();
            // Transfer token fails if it does not exists.
            assert_eq!(erc721.transfer(accounts.bob, 2), Err(Error::TokenNotFound));
            // Token Id 2 does not exists.
            assert_eq!(erc721.owner_of(2), None);
            // Create token Id 2.
            assert_eq!(erc721.mint(2), Ok(()));
            // Alice owns 1 token.
            assert_eq!(erc721.balance_of(accounts.alice), 1);
            // Token Id 2 is owned by Alice.
            assert_eq!(erc721.owner_of(2), Some(accounts.alice));
            // Set Bob as caller
            set_caller(accounts.bob);
            // Bob cannot transfer not owned tokens.
            assert_eq!(erc721.transfer(accounts.eve, 2), Err(Error::NotApproved));
        }

        #[ink::test]
        fn approved_transfer_works() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = LipToken::new();
            // Create token Id 1.
            assert_eq!(erc721.mint(1), Ok(()));
            // Token Id 1 is owned by Alice.
            assert_eq!(erc721.owner_of(1), Some(accounts.alice));
            // Approve token Id 1 transfer for Bob on behalf of Alice.
            assert_eq!(erc721.approve(accounts.bob, 1), Ok(()));
            // Set Bob as caller
            set_caller(accounts.bob);
            // Bob transfers token Id 1 from Alice to Eve.
            assert_eq!(
                erc721.transfer_from(accounts.alice, accounts.eve, 1),
                Ok(())
            );
            // TokenId 3 is owned by Eve.
            assert_eq!(erc721.owner_of(1), Some(accounts.eve));
            // Alice does not owns tokens.
            assert_eq!(erc721.balance_of(accounts.alice), 0);
            // Bob does not owns tokens.
            assert_eq!(erc721.balance_of(accounts.bob), 0);
            // Eve owns 1 token.
            assert_eq!(erc721.balance_of(accounts.eve), 1);
        }

        #[ink::test]
        fn approved_for_all_works() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = LipToken::new();
            // Create token Id 1.
            assert_eq!(erc721.mint(1), Ok(()));
            // Create token Id 2.
            assert_eq!(erc721.mint(2), Ok(()));
            // Alice owns 2 tokens.
            assert_eq!(erc721.balance_of(accounts.alice), 2);
            // Approve token Id 1 transfer for Bob on behalf of Alice.
            assert_eq!(erc721.set_approval_for_all(accounts.bob, true), Ok(()));
            // Bob is an approved operator for Alice
            assert!(erc721.is_approved_for_all(accounts.alice, accounts.bob));
            // Set Bob as caller
            set_caller(accounts.bob);
            // Bob transfers token Id 1 from Alice to Eve.
            assert_eq!(
                erc721.transfer_from(accounts.alice, accounts.eve, 1),
                Ok(())
            );
            // TokenId 1 is owned by Eve.
            assert_eq!(erc721.owner_of(1), Some(accounts.eve));
            // Alice owns 1 token.
            assert_eq!(erc721.balance_of(accounts.alice), 1);
            // Bob transfers token Id 2 from Alice to Eve.
            assert_eq!(
                erc721.transfer_from(accounts.alice, accounts.eve, 2),
                Ok(())
            );
            // Bob does not own tokens.
            assert_eq!(erc721.balance_of(accounts.bob), 0);
            // Eve owns 2 tokens.
            assert_eq!(erc721.balance_of(accounts.eve), 2);
            // Remove operator approval for Bob on behalf of Alice.
            set_caller(accounts.alice);
            assert_eq!(erc721.set_approval_for_all(accounts.bob, false), Ok(()));
            // Bob is not an approved operator for Alice.
            assert!(!erc721.is_approved_for_all(accounts.alice, accounts.bob));
        }

        #[ink::test]
        fn not_approved_transfer_should_fail() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = LipToken::new();
            // Create token Id 1.
            assert_eq!(erc721.mint(1), Ok(()));
            // Alice owns 1 token.
            assert_eq!(erc721.balance_of(accounts.alice), 1);
            // Bob does not owns tokens.
            assert_eq!(erc721.balance_of(accounts.bob), 0);
            // Eve does not owns tokens.
            assert_eq!(erc721.balance_of(accounts.eve), 0);
            // Set Eve as caller
            set_caller(accounts.eve);
            // Eve is not an approved operator by Alice.
            assert_eq!(
                erc721.transfer_from(accounts.alice, accounts.frank, 1),
                Err(Error::NotApproved)
            );
            // Alice owns 1 token.
            assert_eq!(erc721.balance_of(accounts.alice), 1);
            // Bob does not owns tokens.
            assert_eq!(erc721.balance_of(accounts.bob), 0);
            // Eve does not owns tokens.
            assert_eq!(erc721.balance_of(accounts.eve), 0);
        }

        #[ink::test]
        fn burn_works() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = LipToken::new();
            // Create token Id 1 for Alice
            assert_eq!(erc721.mint(1), Ok(()));
            // Alice owns 1 token.
            assert_eq!(erc721.balance_of(accounts.alice), 1);
            // Alice owns token Id 1.
            assert_eq!(erc721.owner_of(1), Some(accounts.alice));
            // Destroy token Id 1.
            assert_eq!(erc721.burn(1), Ok(()));
            // Alice does not owns tokens.
            assert_eq!(erc721.balance_of(accounts.alice), 0);
            // Token Id 1 does not exists
            assert_eq!(erc721.owner_of(1), None);
        }

        #[ink::test]
        fn burn_fails_token_not_found() {
            // Create a new contract instance.
            let mut erc721 = LipToken::new();
            // Try burning a non existent token
            assert_eq!(erc721.burn(1), Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn burn_fails_not_owner() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = LipToken::new();
            // Create token Id 1 for Alice
            assert_eq!(erc721.mint(1), Ok(()));
            // Try burning this token with a different account
            set_caller(accounts.eve);
            assert_eq!(erc721.burn(1), Err(Error::NotOwner));
        }

        fn set_caller(sender: AccountId) {
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(sender);
        }
    }
}