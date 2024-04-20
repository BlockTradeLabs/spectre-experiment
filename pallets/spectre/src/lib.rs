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

use frame_support::{pallet_prelude::*, traits::fungible, Blake2_128Concat};
use sp_arithmetic::Permill;
use sp_core::H256;
use sp_std::vec;
use sp_std::vec::Vec;
use sp_trie::{read_trie_value, verify_trie_proof, LayoutV1, MemoryDB, StorageProof, TrieDB};
use frame_support::sp_runtime::traits::StaticLookup;
use orml_xtokens;
use orml_tokens;
use orml_asset_registry;

use util::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

    use frame_support::sp_runtime::{traits::BlakeTwo256, MultiAddress};
    use frame_system::{
        ensure_none, ensure_signed,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };

    use crate::*;

    pub type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<
        <T as frame_system::Config>::AccountId,
    >>::Balance;
    pub type AccountIdFor<T> = <T as frame_system::Config>::AccountId;
    
    #[pallet::config]
    pub trait Config: frame_system::Config + orml_asset_registry::module::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type NativeBalance: fungible::Inspect<AccountIdFor<Self>>
            + fungible::Mutate<AccountIdFor<Self>>
            + fungible::hold::Inspect<AccountIdFor<Self>>
            + fungible::hold::Mutate<AccountIdFor<Self>>
            + fungible::freeze::Inspect<AccountIdFor<Self>>
            + fungible::freeze::Mutate<AccountIdFor<Self>>;

        // TODO!
        // Add trait associated type for asset functionalities

        /// Trait for allocating funds from pool to trader
        type CapitalAllocator: CapitalAllocator<Self>;
        /// Trait for verifying trade execution done in a foreign Dex
        type TradeExecutionVerifier: TradeExecutionVerifier<Self>;
        /// Constant: Percentage ownership for investor
        #[pallet::constant]
        type InvestorPoolOwnership: Get<u8>;
        /// Constant: Percentage ownrship for trader
        #[pallet::constant]
        type TraderPoolOwnership: Get<u8>;
        /// Number of assets supported 
        #[pallet::constant]
        type TotalSupportedAssets: Get<u8>;
        /// Default Asset Id used to initialize capital pool storage
        type DefaultAsset: Get<Self::AssetId>;
        /// Default Capital Pool Balance
        type DefaultBalance: Get<Self::Balance>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

   
    #[pallet::storage]
    #[pallet::unbounded]
    pub type InvestorProfiles<T: Config> =
        CountedStorageMap<_, Blake2_128Concat, AccountIdFor<T>, InvestorProfile<T>>;

    #[pallet::storage]
    pub type TraderProfiles<T: Config> =
        CountedStorageMap<_, Blake2_128Concat, AccountIdFor<T>, TraderProfile<T>>;

    /// A mapping of Trader Soverign Account to the Onchain Trading Account
    #[pallet::storage]
    pub type OnChainTradingAccounts<T: Config> =
        CountedStorageMap<_, Blake2_128Concat, AccountIdFor<T>, TradingAccounts<AccountIdFor<T>>>;

    /// A mapping of asset id to capital pool
    #[pallet::storage]
    pub type CapitalPool<T: Config> = StorageMap<_,Twox64Concat,T::AssetId,InvestorCapitalPool<T>>;

    /// Relayer account that is responsible for submitting txn for registering trader account and onchain trading account
    /// relating to the trader generated onchain from the contract
    #[pallet::storage]
    pub type Relayer<T: Config> = StorageValue<_, AccountIdFor<T>, OptionQuery>;

    // Genesis Config for `Relayer` storage
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub relayer: Option<AccountIdFor<T>>,
        pub supported_assets: Vec<T::AssetId>,
        pub initial_capital: u128,
        pub initial_blocktime_pool: BlockNumberFor<T>,
        pub fee: u8,
        pub pool_account_id: AccountIdFor<T>
    }

    // impl<T: Config> Default for GenesisConfig<T> {
    //     fn default() -> Self {
    //         Self {
    //             relayer: None
    //         }
    //     }
    // }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            Relayer::<T>::put(
                self.relayer
                    .clone()
                    .expect(" Setup spectre relayer account"),
            
            );
            self.supported_assets.iter().for_each(|asset|{
                let value = InvestorCapitalPool {
                    asset_name: asset.clone(),
                    total_capital: self.initial_capital, 
                    remaining_capital: self.initial_capital, 
                    total_allocated_capital: self.initial_capital, 
                    unrealized_balance: self.initial_capital, 
                    created_at: self.initial_blocktime_pool, 
                    fee: self.fee, 
                    account_id: self.pool_account_id.clone()
                };

                CapitalPool::<T>::insert(asset,value);
            })
            
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
        /// This error should not occur as the relayer is set in the pallet genesis storage
        RelayerUnavailable,
        /// If the relayer submitting the transaction from the contract is not recognized in the chain
        RelayerNotRegistered,
        /// Failed to verify inclusion of buy and sell trade transaction and state execution result
        InvalidProofSubmission,
        /// Failed to update capital balance
        FailedToAddCapital
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        InvestorRegistered,
        TraderRegistered {
            id: AccountIdFor<T>,
        },
        TradeVerifiedSuccesfully {
            trader_id: AccountIdFor<T>,
            onchain_trading_account: AccountIdFor<T>,
            network: Networks,
        },
        FundsAllocated {
            trader_id: AccountIdFor<T>,
            onchain_trading_account: AccountIdFor<T>,
            network: Networks,
        },
    }

    // unsigned transaction for submitting trade execution proofs
    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        // empty pre-dispatch do we don't modify storage
        fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
            Ok(())
        }

        fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            let (trader_id, network, trade_execution_proof, trade_action) = match call {
                Call::verify_trade {
                    trader_id,
                    network,
                    trade_execution_proof,
                    trade_action,
                } => (trader_id, network, trade_execution_proof, trade_action),
                _ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
            };

            // verify proofs submitted per the network
            T::TradeExecutionVerifier::verify_trade_execution(
                trader_id.clone(),
                network.clone(),
                trade_execution_proof.clone(),
                trade_action.clone(),
            )
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {

        /// This extrinsic registers investor by depositing capital to the pool and registering the details in `InvestorProfile`
        /// As the Investor can register in atmost 4 pools supporting assets
        /// Calling this function by specifying the asset registers to the specific pool, if the investors did register, it will add the assets.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::default())]
        pub fn register_investor(
            origin: OriginFor<T>,
            asset: T::AssetId,
            capital_amount: T::Balance,
        ) -> DispatchResult {
            let investor = ensure_signed(origin)?;
            // check if the pool is available
            // This logic of initializing the pool after investor depositing is subjected to change once
            // I find a way to initialize T::AssetId parameter
            // CapitalPool::<T>::try_mutate(asset.into(),|pool|{
            //     // update the pool & investor profile with correct ownership
            //     // transfer from investor to pool
            //     let pool_id_source = T::Lookup::unlookup(pool.account_id);

            //     let blocknumber = <frame_system::Pallet<T>>::block_number();

            //     // let deposited_capital = BoundedBTreeMap::from()
            //     // let mut investor_profile = InvestorProfile {
            //     //     deposited_capital:
            //     // };
            //     // investor_profile::add_capital()

            //      // schedule the actual depositing of asset
            //     <pallet_assets::Pallet::<T>>::transfer(origin, asset, pool_id_source, capital_amount)?;
            // });
                

            Ok(())
        
        }

        #[pallet::call_index(1)]
        #[pallet::weight(Weight::default())]
        pub fn register_trader(
            origin: OriginFor<T>,
            trader_id: AccountIdFor<T>,
            onchain_trading_accounts: TradingAccounts<AccountIdFor<T>>,
        ) -> DispatchResult {
            let relayer_id = ensure_signed(origin)?;
            // check the signer relayer is registered on chain
            let registered_relayer_id =
                Relayer::<T>::get().ok_or(Error::<T>::RelayerUnavailable)?;
            ensure!(
                relayer_id == registered_relayer_id,
                Error::<T>::RelayerNotRegistered
            );

            // register the accounts
            OnChainTradingAccounts::<T>::insert(trader_id.clone(), &onchain_trading_accounts);

            Self::deposit_event(Event::TraderRegistered { id: trader_id });
            Ok(())
        }

        /// Allocate capital from the pool to the trader onchain trading account
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::default())]
        pub fn allocate_capital(origin: OriginFor<T>, network: Networks) -> DispatchResult {
            let trader_id = ensure_signed(origin)?;
            T::CapitalAllocator::allocate_capital(network, trader_id)?;
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(Weight::default())]
        pub fn verify_trade(
            origin: OriginFor<T>,
            trader_id: AccountIdFor<T>,
            network: Networks,
            trade_execution_proof: TradeExecutionProof<BlockNumberFor<T>>,
            trade_action: TradeAction,
        ) -> DispatchResult {
            ensure_none(origin)?;
            <Pallet<T> as ValidateUnsigned>::validate_unsigned(
                TransactionSource::External,
                &Call::verify_trade {
                    trader_id,
                    network,
                    trade_execution_proof,
                    trade_action,
                },
            )
            .map_err(|_| Error::<T>::InvalidProofSubmission)?;
            Ok(())
        }
    }
}
