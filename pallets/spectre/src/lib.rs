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

use util::*;


pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

    use frame_system::pallet_prelude::OriginFor;

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
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type InvestorProfiles<T: Config> = CountedStorageMap<_,Blake2_128Concat,T::AccountId,InvestorProfile<T>>;

    #[pallet::storage]
    pub type TraderProfiles<T: Config> = CountedStorageMap<_,Blake2_128Concat,T::AccountId,TraderProfile<T>>;

    /// A mapping of Trader Soverign Account to the Onchain Trading Account
    #[pallet::storage]
    pub type OnChainTradingAccount<T: Config> = CountedStorageMap<_,Blake2_128Concat,T::AccountId,TradingAccounts<T>>;

    #[pallet::storage]
    pub type CapitalPool<T: Config> = StorageValue<_,InvestorLP<T>>;

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

    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        InvestorRegistered {
            capital_bond: BalanceOf<T>,
        },
        TraderRegistered {
            amount_bond: BalanceOf<T>
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
        pub fn register_trader(origin:OriginFor<T>) -> DispatchResult {
            Ok(())   
        }

        /// Allocate capital from the pool to the trader onchain trading account
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::default())]
        pub fn allocate_capital(origin:OriginFor<T>) -> DispatchResult {
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(Weight::default())]
        pub fn verify_trade(origin:OriginFor<T>) -> DispatchResult {
            Ok(())
        }
    }
}