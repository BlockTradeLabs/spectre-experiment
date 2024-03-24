//! Spectre pallet.
//!
//! Terminologies
//! 1. OnChain Trading account -> A keyPair stored privately in phala contract that is linked with trader sovereign keypair, responsible
//!                               for signing trade transactions
//!
//! 2. Pool -> Liquidity Pool, in this context is just a pool holding and keeping track of deposited tokens from investors,
//!         note that there is no swapping functionalities
//!
//!
//! Main functionalities are;
//!
//!     1. Investor depositing and registering
//!
//!     2. LP tracking for deposited funds
//!
//!     3. Funds allocation from LP to traders
//!
//!     4. Fetching live price feeds from oracle
//!
//!     5. Trader executing trades
//!         a. Signing transaction payload
//!         b. Sending to Relayer
//!
//!     6.  Relayer updating transaction execution to the oracle.
//!
//!     7. Tracking trader perfomance
//!
//!

#![cfg_attr(not(feature = "std"), no_std)]

pub mod util;

use frame_support::{
    pallet_prelude::*,
    traits::fungible,
    Blake2_128Concat,
};
use sp_arithmetic::{Permill};
use sp_trie::{read_trie_value, MemoryDB, TrieDB};
use sp_core::H256;

use util::*;


pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

    use frame_system::{ensure_none, ensure_signed, pallet_prelude::{BlockNumberFor, OriginFor}};

    use crate::*;
   

    pub type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<
	<T as frame_system::Config>::AccountId,
    >>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {

        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type NativeBalance: fungible::Inspect<Self::AccountId>
			+ fungible::Mutate<Self::AccountId>
			+ fungible::hold::Inspect<Self::AccountId>
			+ fungible::hold::Mutate<Self::AccountId>
			+ fungible::freeze::Inspect<Self::AccountId>
			+ fungible::freeze::Mutate<Self::AccountId>;

        type CapitalAllocator: CapitalAllocator<Self>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type InvestorProfiles<T: Config> = CountedStorageMap<_,Blake2_128Concat,T::AccountId,InvestorProfile<T>>;

    #[pallet::storage]
    pub type TraderProfiles<T: Config> = CountedStorageMap<_,Blake2_128Concat,T::AccountId,TraderProfile<T>>;

    /// A mapping of Trader Soverign Account to the Onchain Trading Account
    #[pallet::storage]
    pub type OnChainTradingAccounts<T: Config> = CountedStorageMap<_,Blake2_128Concat,T::AccountId,TradingAccounts<T::AccountId>>;

    #[pallet::storage]
    pub type CapitalPool<T: Config> = StorageValue<_,InvestorLP<T>>;

    /// Relayer account that is responsible for submitting txn for registering trader account and onchain trading account
    /// relating to the trader generated onchain from the contract
    #[pallet::storage]
    pub type Relayer<T:Config> = StorageValue<_,T::AccountId,OptionQuery>;

    // Genesis Config for `Relayer` storage
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        relayer: T::AccountId
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self){
            Relayer::<T>::put(self.relayer.clone())
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Returned if the pool fails to allocate funds to the trader.
        FailedToAllocateFundsToTrader,
        /// If the trader account is not registered
        UnregisteredTraderAcount,
        /// Returned if an operation on creating account key pair for the on chain trading account fails
        FailedToGenerateTradingAccount,
        /// Returned if failed to transfer funds from the contract account to on chain trading account
        FailedToAllocateFunds,
        /// Returned if not enough bond placed by the trader when registering.
        InsufficientBond,
        /// Returned if the proof of trade execution is invalid
        FailedTradeProof,
        /// Returned if the account is trying to re register
        AccountAlreadtRegistered,
        /// Error returned if we fail to get value from proof nodes
        FailedToFetchValueFromTrie,
        /// Error returned if the value obtained from trie node fails to decode
        FailedToDecodeValue,
        /// Returned if not enough deposit fund from investor from registering.
        InsufficientDeposit,

        AccountUnavailable,

        RelayerUnavailable, // This error should not occur as the relayer is set in the pallet genesis storage

        RelayerNotRegistered,

        InvalidProofSubmission
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        InvestorRegistered {
            capital_bond: BalanceOf<T>,
        },
        TraderRegistered {
            id: T::AccountId
        },
        TradeVerifiedSuccesfully {
            trader_id: T::AccountId,
            onchain_trading_account: T::AccountId,
            network: Networks
        },
        FundsAllocated {
            trader_id: T::AccountId,
            onchain_trading_account: T::AccountId,
            network: Networks
        }
    }

     // unsigned transaction for submitting trade execution proofs
     #[pallet::validate_unsigned]
     impl<T: Config> ValidateUnsigned for Pallet<T>{

        type Call = Call<T>;

        // empty pre-dispatch do we don't modify storage
        fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
            Ok(())
        }

        fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity{

           let (network, trade_execution_proof) = match call {
                Call::verify_trade { network, trade_execution_proof} => (network, trade_execution_proof),
                _ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
            };

            // verify proofs submitted per the network

            Ok(ValidTransaction{
                priority: 100,
                requires: vec![],
                provides: vec![],
                longevity: TransactionLongevity::MAX,
                propagate: true,
            })
        } 
    }

    // 1. Learning
    // 2. (Maths & Cryptography & Blockchain )

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::default())]
        pub fn register_investor(origin:OriginFor<T>) -> DispatchResult {
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(Weight::default())]
        pub fn register_trader(origin:OriginFor<T>, trader_id: T::AccountId, onchain_trading_accounts: TradingAccounts<T::AccountId>) -> DispatchResult {
            let relayer_id  = ensure_signed(origin)?;
            // check the signer relayer is registered on chain
            let registered_relayer_id = Relayer::<T>::get().ok_or(Error::<T>::RelayerUnavailable)?;
            ensure!(relayer_id == registered_relayer_id, Error::<T>::RelayerNotRegistered);

            // register the accounts
            OnChainTradingAccounts::<T>::insert(trader_id.clone(),&onchain_trading_accounts);

            Self::deposit_event(
                Event::TraderRegistered {
                    id: trader_id
                }
            );
            Ok(())   
        }

        /// Allocate capital from the pool to the trader onchain trading account
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::default())]
        pub fn allocate_capital(origin:OriginFor<T>, network:Networks) -> DispatchResult {
            let trader_id = ensure_signed(origin)?;
            T::CapitalAllocator::allocate_capital(network, trader_id)?;
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(Weight::default())]
        pub fn verify_trade(origin:OriginFor<T>, network: Networks, trade_execution_proof: TradeExecutionProof<BlockNumberFor<T>>) -> DispatchResult {
            ensure_none(origin)?;
            <Pallet<T> as ValidateUnsigned>::validate_unsigned(
                TransactionSource::External,
                &Call::verify_trade { network, trade_execution_proof},
            )
            .map_err(|_| Error::<T>::InvalidProofSubmission)?;
            Ok(())
        }
    }

   
}