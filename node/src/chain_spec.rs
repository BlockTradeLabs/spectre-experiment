use {
    cumulus_primitives_core::{Junction, MultiLocation, ParaId},
    fp_evm::GenesisAccount,
    frame_benchmarking::__private::traits::ConstU32,
    hex_literal::hex,
    orml_asset_registry::AssetMetadata,
    parity_scale_codec::Encode,
    sc_chain_spec::{ChainSpecExtension, ChainSpecGroup},
    sc_network::config::MultiaddrWithPeerId,
    sc_service::ChainType,
    serde::{Deserialize, Serialize},
    sp_core::bounded_vec::BoundedVec,
    spectre_runtime::{
        AccountId, Balance, EVMChainIdConfig, EVMConfig, MaintenanceModeConfig, MigrationsConfig,
        PolkadotXcmConfig, Precompiles,
    },
    staging_xcm::VersionedMultiLocation,
};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec =
    sc_service::GenericChainSpec<spectre_runtime::RuntimeGenesisConfig, Extensions>;

/// Orcherstrator's parachain id
pub const ORCHESTRATOR: ParaId = ParaId::new(1000);

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

pub fn development_config(para_id: ParaId, boot_nodes: Vec<String>) -> ChainSpec {
    // Give your base currency a unit name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "UNIT".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());
    properties.insert("isEthereum".into(), true.into());

    let mut default_funded_accounts = pre_funded_accounts();
    default_funded_accounts.sort();
    default_funded_accounts.dedup();
    let boot_nodes: Vec<MultiaddrWithPeerId> = boot_nodes
        .into_iter()
        .map(|x| {
            x.parse::<MultiaddrWithPeerId>()
                .unwrap_or_else(|e| panic!("invalid bootnode address format {:?}: {:?}", x, e))
        })
        .collect();

    ChainSpec::builder(
        spectre_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
        Extensions {
            relay_chain: "rococo-local".into(), // You MUST set this to the correct network!
            para_id: para_id.into(),
        },
    )
    .with_name("Development")
    .with_id("dev")
    .with_chain_type(ChainType::Development)
    .with_genesis_config(testnet_genesis(
        default_funded_accounts.clone(),
        para_id,
        AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")), // Alith
    ))
    .with_properties(properties)
    .with_boot_nodes(boot_nodes)
    .build()
}

pub fn local_testnet_config(para_id: ParaId, boot_nodes: Vec<String>) -> ChainSpec {
    // Give your base currency a unit name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "UNIT".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 42.into());
    properties.insert("isEthereum".into(), true.into());
    let protocol_id = format!("container-chain-{}", para_id);

    let mut default_funded_accounts = pre_funded_accounts();
    default_funded_accounts.sort();
    default_funded_accounts.dedup();
    let boot_nodes: Vec<MultiaddrWithPeerId> = boot_nodes
        .into_iter()
        .map(|x| {
            x.parse::<MultiaddrWithPeerId>()
                .unwrap_or_else(|e| panic!("invalid bootnode address format {:?}: {:?}", x, e))
        })
        .collect();

    ChainSpec::builder(
        spectre_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
        Extensions {
            relay_chain: "rococo-local".into(), // You MUST set this to the correct network!
            para_id: para_id.into(),
        },
    )
    .with_name(&format!("Spectre Finance {}", para_id))
    .with_id(&format!("spectre_finance_{}", para_id))
    .with_chain_type(ChainType::Local)
    .with_genesis_config(testnet_genesis(
        default_funded_accounts.clone(),
        para_id,
        AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")), // Alith
    ))
    .with_properties(properties)
    .with_protocol_id(&protocol_id)
    .with_boot_nodes(boot_nodes)
    .build()
}

fn testnet_genesis(
    endowed_accounts: Vec<AccountId>,
    id: ParaId,
    root_key: AccountId,
) -> serde_json::Value {
    // This is the simplest bytecode to revert without returning any data.
    // We will pre-deploy it under all of our precompiles to ensure they can be called from
    // within contracts.
    // (PUSH1 0x00 PUSH1 0x00 REVERT)
    let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];

    // asset metadata
    // let sf_dot = AssetMetadata::<Balance, Vec<u8>, ConstU32<10>> {
    //     decimals: 12,
    //     name: BoundedVec::truncate_from("sfDOT".as_bytes().to_vec()),
    //     symbol: BoundedVec::truncate_from("sfDOT".as_bytes().to_vec()),
    //     existential_deposit: 0,
    //     location: Some(VersionedMultiLocation::V3(MultiLocation::new(
    //         0,
    //         Junction::GeneralIndex(1),
    //     ))),
    //     additional: b"spectre finance derived token".to_vec(),
    // };
    // let sf_usdt = AssetMetadata::<Balance, Vec<u8>, ConstU32<10>> {
    //     decimals: 12,
    //     name: BoundedVec::truncate_from("sfUSDT".as_bytes().to_vec()),
    //     symbol: BoundedVec::truncate_from("sfUSDT".as_bytes().to_vec()),
    //     existential_deposit: 0,
    //     location: Some(VersionedMultiLocation::V3(MultiLocation::new(
    //         0,
    //         Junction::GeneralIndex(2),
    //     ))),
    //     additional: b"spectre finance derived token".to_vec(),
    // };
    // let sf_usdc = AssetMetadata::<Balance, Vec<u8>, ConstU32<10>> {
    //     decimals: 12,
    //     name: BoundedVec::truncate_from("sfUSDC".as_bytes().to_vec()),
    //     symbol: BoundedVec::truncate_from("sUSDC".as_bytes().to_vec()),
    //     existential_deposit: 0,
    //     location: Some(VersionedMultiLocation::V3(MultiLocation::new(
    //         0,
    //         Junction::GeneralIndex(3),
    //     ))),
    //     additional: b"spectre finance derived token".to_vec(),
    // };

    let g = spectre_runtime::RuntimeGenesisConfig {
        system: Default::default(),
        balances: spectre_runtime::BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1 << 80))
                .collect(),
        },
        parachain_info: spectre_runtime::ParachainInfoConfig {
            parachain_id: id,
            ..Default::default()
        },
        parachain_system: Default::default(),
        // EVM compatibility
        // We should change this to something different than Moonbeam
        // For now moonwall is very tailored for moonbeam so we need it for tests
        evm_chain_id: EVMChainIdConfig {
            chain_id: 1281u32 as u64,
            ..Default::default()
        },
        evm: EVMConfig {
            // We need _some_ code inserted at the precompile address so that
            // the evm will actually call the address.
            ..Default::default()
        },
        ethereum: Default::default(),
        base_fee: Default::default(),
        transaction_payment: Default::default(),
        sudo: spectre_runtime::SudoConfig {
            key: Some(root_key),
        },
        authorities_noting: spectre_runtime::AuthoritiesNotingConfig {
            orchestrator_para_id: ORCHESTRATOR,
            ..Default::default()
        },
        migrations: MigrationsConfig {
            ..Default::default()
        },
        maintenance_mode: MaintenanceModeConfig {
            start_in_maintenance_mode: false,
            ..Default::default()
        },
        // This should initialize it to whatever we have set in the pallet
        polkadot_xcm: PolkadotXcmConfig {
            _config: Default::default(),
            safe_xcm_version: Some(2),
        },
        tx_pause: Default::default(),
        asset_registry: Default::default(),
        assets: spectre_runtime::AssetsConfig { balances: vec![] },
        spectre: spectre_runtime::SpectreConfig {
            relayer: Some(root_key),
            initial_capital: 0,
            supported_assets: vec![],
            fee: 10, // percentage
        },
    };

    serde_json::to_value(g).unwrap()
}

/// Get pre-funded accounts
pub fn pre_funded_accounts() -> Vec<AccountId> {
    // These addresses are derived from Substrate's canonical mnemonic:
    // bottom drive obey lake curtain smoke basket hold race lonely fit walk
    vec![
        AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")), // Alith
        AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0")), // Baltathar
        AccountId::from(hex!("798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc")), // Charleth
        AccountId::from(hex!("773539d4Ac0e786233D90A233654ccEE26a613D9")), // Dorothy
    ]
}
