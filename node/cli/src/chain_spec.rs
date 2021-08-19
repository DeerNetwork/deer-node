// This file is part of Substrate.

// Copyright (C) 2018-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Substrate chain configurations.

use grandpa_primitives::AuthorityId as GrandpaId;
use hex_literal::hex;
use node_runtime::{
	constants::currency::*, wasm_binary_unwrap, AuthorityDiscoveryConfig, BabeConfig,
	BalancesConfig, Block, CouncilConfig, DemocracyConfig, ElectionsConfig, GrandpaConfig,
	ImOnlineConfig, IndicesConfig, SessionConfig, SessionKeys, StakerStatus, StakingConfig,
	SudoConfig, SystemConfig, TechnicalCommitteeConfig, MAX_NOMINATIONS,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_chain_spec::{ChainSpecExtension, Properties};
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

pub use node_primitives::{AccountId, Balance, Signature};
pub use node_runtime::GenesisConfig;

type AccountPublic = <Signature as Verify>::Signer;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<Block>,
	/// The light sync state extension used by the sync-state rpc.
	pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

fn session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
	SessionKeys { grandpa, babe, im_online, authority_discovery }
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(
	seed: &str,
) -> (AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	initial_nominators: Vec<AccountId>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> GenesisConfig {
	let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
			get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
			get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
		]
	});
	// endow all authorities and nominators.
	initial_authorities
		.iter()
		.map(|x| &x.0)
		.chain(initial_nominators.iter())
		.for_each(|x| {
			if !endowed_accounts.contains(&x) {
				endowed_accounts.push(x.clone())
			}
		});

	// stakers: all validators and nominators.
	let mut rng = rand::thread_rng();
	let stakers = initial_authorities
		.iter()
		.map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
		.chain(initial_nominators.iter().map(|x| {
			use rand::{seq::SliceRandom, Rng};
			let limit = (MAX_NOMINATIONS as usize).min(initial_authorities.len());
			let count = rng.gen::<usize>() % limit;
			let nominations = initial_authorities
				.as_slice()
				.choose_multiple(&mut rng, count)
				.into_iter()
				.map(|choice| choice.0.clone())
				.collect::<Vec<_>>();
			(x.clone(), x.clone(), STASH, StakerStatus::Nominator(nominations))
		}))
		.collect::<Vec<_>>();

	let num_endowed_accounts = endowed_accounts.len();

	const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
	const STASH: Balance = ENDOWMENT / 1000;

	GenesisConfig {
		system: SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect(),
		},
		indices: IndicesConfig { indices: vec![] },
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
					)
				})
				.collect::<Vec<_>>(),
		},
		staking: StakingConfig {
			validator_count: initial_authorities.len() as u32,
			minimum_validator_count: initial_authorities.len() as u32,
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers,
			..Default::default()
		},
		democracy: DemocracyConfig::default(),
		elections: ElectionsConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.map(|member| (member, STASH))
				.collect(),
		},
		council: CouncilConfig::default(),
		technical_committee: TechnicalCommitteeConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			phantom: Default::default(),
		},
		sudo: SudoConfig { key: root_key },
		babe: BabeConfig {
			authorities: vec![],
			epoch_config: Some(node_runtime::BABE_GENESIS_EPOCH_CONFIG),
		},
		im_online: ImOnlineConfig { keys: vec![] },
		authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
		grandpa: GrandpaConfig { authorities: vec![] },
		technical_membership: Default::default(),
		treasury: Default::default(),
		vesting: Default::default(),
		file_storage: Default::default(),
		transaction_storage: Default::default(),
	}
}

fn development_config_genesis() -> GenesisConfig {
	testnet_genesis(
		vec![authority_keys_from_seed("Alice")],
		vec![],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		development_config_genesis,
		vec![],
		None,
		None,
		None,
		Default::default(),
	)
}

fn local_testnet_genesis() -> GenesisConfig {
	testnet_genesis(
		vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
		vec![],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Local Testnet",
		"local_testnet",
		ChainType::Local,
		local_testnet_genesis,
		vec![],
		Some(
			TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
				.expect("Staging telemetry url is valid; qed"),
		),
		None,
		None,
		Default::default(),
	)
}

fn deer_testnet_genesis() -> GenesisConfig {
	// ./scripts/prepare-test-net.sh 4
	let initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)> = vec![
		(
			//5CoeCei9ULeUXKKRY4MDq3Bpp2JH5JVdRHfU4B2UZsKaGtNm
			hex!["20bf3f380a5d4888e2f7c467bb03abf0efd8780c4ea0cdf8b1ec2e70d24e2249"].into(),
			//5HdL6KCaMwxbAR3j7wY25DW3HcrbxQLXpLqGPCupec2Fne2L
			hex!["f60effbef7654a6590fca2b94edc9fcd80434aa08ad3ecf8c9514851cbe5057a"].into(),
			//5FhWYTtETbby4YjY648GUaZmjCKwUj26HSfbNUH46yQ35Jzj
			hex!["a0c6b1c6dcc92bd6adf97c71f0033e343ee525bb03af50b6481823aca2cc6574"]
				.unchecked_into(),
			//5FvWhH6E1nhm4hLBCxdETrbCUksYR2LZSypJDEiM6hMjxB94
			hex!["aab16570e0ff1a8d71f77c70e33d04f165e54e6035a4db8aaf7dc131b4484276"]
				.unchecked_into(),
			//5FvWhH6E1nhm4hLBCxdETrbCUksYR2LZSypJDEiM6hMjxB94
			hex!["aab16570e0ff1a8d71f77c70e33d04f165e54e6035a4db8aaf7dc131b4484276"]
				.unchecked_into(),
			//5FvWhH6E1nhm4hLBCxdETrbCUksYR2LZSypJDEiM6hMjxB94
			hex!["aab16570e0ff1a8d71f77c70e33d04f165e54e6035a4db8aaf7dc131b4484276"]
				.unchecked_into(),
		),
		(
			//5DAmdPpeV7SPB7eKb8UbBBrTMfKGrGrVCtZCu7rsPjzksHHt
			hex!["30dc668ff7fb462ed059f5de128f46654f71858b38e22f0cf64a53158783ca1d"].into(),
			//5EqKaozw6DP3QrCnmjgrr1S5kWAvJqrCZyEtMq3M6bGszGKQ
			hex!["7a7f87f82846340224af31c6b5305bdd332ce93eae157c2b1aaa9b58fc1ef374"].into(),
			//5H6nw468ppCq1B3EcKuCXPvUR1QSJsPeBJTpo9Tw1XnBvtpF
			hex!["dec4be7d16bb37269ea06843e79ccd5fe5800744bb907182cc1aa524bd9f9ebf"]
				.unchecked_into(),
			//5CorSAYRNJFqKu1kpT3Sf8wkv7GpLSsTsWRrMmHFmGqD4ZjL
			hex!["20e86d4a7f96b9130aac1d20ba1bc294d54e98b98853ca7ea4f3447c8ad65a23"]
				.unchecked_into(),
			//5CorSAYRNJFqKu1kpT3Sf8wkv7GpLSsTsWRrMmHFmGqD4ZjL
			hex!["20e86d4a7f96b9130aac1d20ba1bc294d54e98b98853ca7ea4f3447c8ad65a23"]
				.unchecked_into(),
			//5CorSAYRNJFqKu1kpT3Sf8wkv7GpLSsTsWRrMmHFmGqD4ZjL
			hex!["20e86d4a7f96b9130aac1d20ba1bc294d54e98b98853ca7ea4f3447c8ad65a23"]
				.unchecked_into(),
		),
	];

	let root_key: AccountId = initial_authorities[0].0.clone();

	let endowed_accounts: Vec<AccountId> = initial_authorities
		.iter()
		.map(|(_, controller, ..)| controller.clone())
		.chain(initial_authorities.iter().map(|(stasher, ..)| stasher.clone()))
		.collect();

	testnet_genesis(initial_authorities, vec![], root_key, Some(endowed_accounts))
}

/// Local tesnet config
pub fn deer_testnet_local_config() -> ChainSpec {
	let boot_nodes = vec![];
	let protocol_id: &str = "deert1";
	let properties = {
		let mut p = Properties::new();
		p.insert("tokenSymbol".into(), "DEER".into());
		p.insert("tokenDecimals".into(), 12.into());
		p.insert("ss58Format".into(), 0.into()); // Will be 88
		p
	};

	ChainSpec::from_genesis(
		"Deer Testnet",
		"deer_testnet",
		ChainType::Live,
		deer_testnet_genesis,
		boot_nodes,
		None,
		Some(protocol_id),
		Some(properties),
		Default::default(),
	)
}

/// Tesnet config
pub fn deer_testnet_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../res/testnet.json")[..])
}

/// Mainnet config
pub fn deer_mainnet_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../res/mainnet.json")[..])
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;
	use sp_runtime::BuildStorage;

	fn local_testnet_genesis_instant_single() -> GenesisConfig {
		testnet_genesis(
			vec![authority_keys_from_seed("Alice")],
			vec![],
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			None,
		)
	}

	/// Local testnet config (single validator - Alice)
	pub fn integration_test_config_with_single_authority() -> ChainSpec {
		ChainSpec::from_genesis(
			"Integration Test",
			"test",
			ChainType::Development,
			local_testnet_genesis_instant_single,
			vec![],
			None,
			None,
			None,
			Default::default(),
		)
	}

	/// Local testnet config (multivalidator Alice + Bob)
	pub fn integration_test_config_with_two_authorities() -> ChainSpec {
		ChainSpec::from_genesis(
			"Integration Test",
			"test",
			ChainType::Development,
			local_testnet_genesis,
			vec![],
			None,
			None,
			None,
			Default::default(),
		)
	}

	#[test]
	fn test_create_development_chain_spec() {
		development_config().build_storage().unwrap();
	}

	#[test]
	fn test_create_local_testnet_chain_spec() {
		local_testnet_config().build_storage().unwrap();
	}

	#[test]
	fn test_deer_test_net_chain_spec() {
		deer_testnet_local_config().build_storage().unwrap();
	}
}
