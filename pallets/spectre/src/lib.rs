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

use {
    frame_support::{
        pallet_prelude::*, sp_runtime::traits::StaticLookup, traits::fungible, Blake2_128Concat,
    },
    orml_asset_registry, orml_tokens, orml_xtokens,
    sp_arithmetic::Permill,
    sp_core::H256,
    sp_std::{vec, vec::Vec},
    sp_trie::{read_trie_value, verify_trie_proof, LayoutV1, MemoryDB, StorageProof, TrieDB},
};

use util::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

    use {
        frame_support::sp_runtime::{traits::BlakeTwo256, MultiAddress},
        frame_system::{
            ensure_none, ensure_signed,
            pallet_prelude::{BlockNumberFor, OriginFor},
            RawOrigin,
        },
    };

    use crate::*;

    pub type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<
        <T as frame_system::Config>::AccountId,
    >>::Balance;
    pub type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

    pub type AssetBalance<T> = <T as orml_tokens::Config>::Balance;

    #[pallet::config]
    pub trait Config:
        frame_system::Config + orml_asset_registry::module::Config + orml_tokens::Config
    {
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
        /// Constant: Withdraw period that should pass for investor to withdraw capital + returns
        type WithdrawPeriod: Get<BlockNumberFor<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::unbounded]
    pub type InvestorProfiles<T: Config> =
        CountedStorageMap<_, Blake2_128Concat, AccountIdFor<T>, InvestorProfile<T>>;

    #[pallet::storage]
    pub type TraderProfiles<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AccountIdFor<T>,
        Blake2_128Concat,
        AccountIdFor<T>,
        TraderProfile<T>,
    >;

    /// A mapping of Trader Soverign Account to the Onchain Trading Account
    #[pallet::storage]
    pub type OnChainTradingAccounts<T: Config> =
        CountedStorageMap<_, Blake2_128Concat, AccountIdFor<T>, TradingAccounts<AccountIdFor<T>>>;

    /// A mapping of asset id to capital pool
    #[pallet::storage]
    pub type CapitalPool<T: Config> =
        StorageMap<_, Twox64Concat, T::CurrencyId, InvestorCapitalPool<T>, ValueQuery>;

    /// Relayer account that is responsible for submitting txn for registering trader account and onchain trading account
    /// relating to the trader generated onchain from the contract
    #[pallet::storage]
    pub type Relayer<T: Config> = StorageValue<_, AccountIdFor<T>, OptionQuery>;

    // Genesis Config for `Relayer` storage
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub relayer: Option<AccountIdFor<T>>,
        pub supported_assets: Vec<T::CurrencyId>,
        pub initial_capital: AssetBalance<T>,
        pub fee: u8,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                relayer: None,
                supported_assets: vec![],
                initial_capital: AssetBalance::<T>::default(),
                fee: 0,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            Relayer::<T>::put(
                self.relayer
                    .clone()
                    .expect(" Setup spectre relayer account"),
            );
            self.supported_assets.iter().for_each(|asset| {
                let account_id = Pallet::<T>::generate_pool_account(asset.clone());
                let investor_pool = InvestorCapitalPool {
                    asset_name: asset.clone(),
                    total_capital: self.initial_capital,
                    remaining_capital: self.initial_capital,
                    total_allocated_capital: self.initial_capital,
                    unrealized_balance: self.initial_capital,
                    fee: self.fee,
                    account_id,
                };

                CapitalPool::<T>::insert(asset, investor_pool);
            });
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Returned if the pool fails to allocate funds to the trader.
        FailedToAllocateFundsToTrader,
        /// If the trader account is not registered
        TraderNotFunded,
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
        /// Returned when trader is not found
        TraderNotRegistered,
        /// Returned when the trade tx is not recognized
        InvalidTxInclusion,
        /// Returned when the reading and verifying onchain trading balance is invalid
        InvalidBalanceStateProof,

        AccountUnavailable,
        /// This error should not occur as the relayer is set in the pallet genesis storage
        RelayerUnavailable,
        /// If the relayer submitting the transaction from the contract is not recognized in the chain
        RelayerNotRegistered,
        /// Failed to verify inclusion of buy and sell trade transaction and state execution result
        InvalidProofSubmission,
        /// Failed to update capital balance
        FailedToAddCapital,
        /// Returned when Asset Pool not currently supported
        AssetPoolNotSupported,
        /// Returned when failed to transfer funds from investor to pool account
        FailedToTransferCapitalToPool,
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
            let (trader_id, asset_id, network, trade_execution_proof, trade_action) = match call {
                Call::verify_trade_execution {
                    trader_id,
                    asset_id,
                    network,
                    trade_execution_proof,
                    trade_action,
                } => (
                    trader_id,
                    asset_id,
                    network,
                    trade_execution_proof,
                    trade_action,
                ),
                _ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
            };

            // Modify this to be dynamic in terms of priority,
            // All polkadot related verification should have lesser priorioty than non polkadot trade verification
            Ok(ValidTransaction {
                priority: u64::MAX,
                requires: vec![],
                provides: vec![],
                longevity: TransactionLongevity::MAX,
                propagate: true,
            })

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
            asset_id: T::CurrencyId,
            capital_amount: AssetBalance<T>,
        ) -> DispatchResult {
            let investor = ensure_signed(origin.clone())?;

            ensure!(
                CapitalPool::<T>::contains_key(asset_id.clone()),
                Error::<T>::AssetPoolNotSupported
            );

            let _ = CapitalPool::<T>::try_mutate(asset_id.clone(), |pool| {
                // update the pool & investor profile with correct ownership
                // transfer from investor to pool
                let pool_id_source = T::Lookup::unlookup(pool.account_id.clone());
                // update pool
                pool.add_capital(capital_amount);
                // update investor profile
                let mut investor_profile = InvestorProfile::<T>::default();
                investor_profile.register_capital(investor.clone(), asset_id, capital_amount);
                // actual depositing of asset
                <orml_tokens::Pallet<T>>::transfer_keep_alive(
                    RawOrigin::Signed(investor).into(),
                    pool_id_source,
                    asset_id,
                    capital_amount,
                )
                .map_err(|_| Error::<T>::FailedToTransferCapitalToPool)?;
                Ok::<(), Error<T>>(())
            });

            Ok(())
        }

        /// Registers trader after generating on chain trading accounts in the contract. 
        /// This extrinsic accept the trading acconts public key to registers them with trader account id
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

            match network {
                Networks::Substrate => {
                    let onchain_trading_account =
                        OnChainTradingAccounts::<T>::get(trader_id.clone())
                            .ok_or(Error::<T>::TraderNotRegistered)?
                            .substrate
                            .ok_or(Error::<T>::TraderNotRegistered)?;

                    T::CapitalAllocator::allocate_capital(
                        network.clone(),
                        trader_id.clone(),
                        onchain_trading_account.clone(),
                    )?;
                    Self::deposit_event(Event::FundsAllocated {
                        trader_id,
                        onchain_trading_account,
                        network,
                    });
                }
                _ => todo!(),
            }

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(Weight::default())]
        pub fn verify_trade_execution(
            origin: OriginFor<T>,
            trader_id: AccountIdFor<T>,
            asset_id: T::CurrencyId,
            network: Networks,
            trade_execution_proof: TradeExecutionProof<BlockNumberFor<T>>,
            trade_action: TradeAction,
        ) -> DispatchResult {
            ensure_none(origin)?;

            let trading_account = OnChainTradingAccounts::<T>::get(trader_id.clone())
                .ok_or(Error::<T>::TraderNotRegistered)?
                .substrate
                .ok_or(Error::<T>::TraderNotRegistered)?;

             // verify proofs submitted per the network
             T::TradeExecutionVerifier::verify_trade_execution(
                trader_id.clone(),
                trading_account.clone(),
                asset_id.clone(),
                network.clone(),
                trade_execution_proof.clone(),
                trade_action.clone(),
            )?;
            
            Self::deposit_event(Event::TradeVerifiedSuccesfully{
                network,
                onchain_trading_account: trading_account,
                trader_id
            });
            Ok(())
        }
    }
}
