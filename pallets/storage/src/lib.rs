//! # Storage Online Module

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

mod constants;

pub use constants::*;

pub mod weights;

use codec::{Decode, Encode};
use frame_support::{
	traits::{
		Currency, ExistenceRequirement, Get, OnUnbalanced, ReservableCurrency, UnixTime,
		WithdrawReasons,
	},
	PalletId,
};
use frame_system::{pallet_prelude::BlockNumberFor, Config as SystemConfig};
use p256::ecdsa::{
	signature::{Signature, Verifier},
	VerifyingKey,
};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, One, Saturating, StaticLookup, Zero},
	Perbill, RuntimeDebug, SaturatedConversion,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

pub type FileId = Vec<u8>;
pub type EnclaveId = Vec<u8>;
pub type PubKey = Vec<u8>;
pub type MachineId = Vec<u8>;
pub type RoundIndex = u32;
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;

type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

pub use pallet::*;
pub use weights::WeightInfo;

// syntactic sugar for logging.
#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: crate::LOG_TARGET,
			concat!("[{:?}] 💸 ", $patter), <frame_system::Pallet<T>>::block_number() $(, $values)*
		)
	};
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct NodeInfo<BlockNumber> {
	/// A increment id of one report
	pub rid: u64,
	/// Effective storage space
	pub used: u64,
	/// Mine power of node, use this to distribute mining rewards
	pub power: u64,
	/// Latest report at
	pub reported_at: BlockNumber,
}

/// Information round rewards
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct RewardInfo<Balance> {
	/// Reward for node power
	pub mine_reward: Balance,
	/// Reward for node store file
	pub store_reward: Balance,
	/// How many mine reward that already assigned to the node
	pub paid_mine_reward: Balance,
	/// How many store reward that already assigned to the node
	pub paid_store_reward: Balance,
}

/// Derive from StoreFile, Record the replicas and expire time
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct FileOrder<AccountId, Balance, BlockNumber> {
	/// The cost of storing for a period of time
	pub fee: Balance,
	/// Store file size
	pub file_size: u64,
	/// When the order need to close or renew
	pub expire_at: BlockNumber,
	/// Nodes store the file
	pub replicas: Vec<AccountId>,
}

/// File that users submit to the network for storage
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct StoreFile<Balance, BlockNumber> {
	/// Funds gathered in this file
	pub reserved: Balance,
	/// Basic cost of sumit to network
	pub base_fee: Balance,
	// Store file size
	pub file_size: u64,
	// When added file
	pub added_at: BlockNumber,
}

/// Information stashing a node
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct StashInfo<AccountId, Balance> {
	/// Stasher account
	pub stasher: AccountId,
	/// Stash funds
	pub deposit: Balance,
	/// Node's machine id
	pub machine_id: Option<MachineId>,
}

/// Information for TEE node
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct RegisterInfo {
	/// PUb key to verify signed message
	pub key: PubKey,
	/// Tee enclave id
	pub enclave: EnclaveId,
}

/// Record node's effictive storage size and power
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct NodeStats {
	/// Node's power
	pub power: u64,
	/// Eeffictive storage size
	pub used: u64,
}

/// Record network's effictive storage size and power
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct SummaryStats {
	/// Network's power
	pub power: u128,
	/// Eeffictive storage size
	pub used: u128,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The module id, used for deriving its sovereign account ID.
		type PalletId: Get<PalletId>;

		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// The Treasury trait.
		type Treasury: OnUnbalanced<NegativeImbalanceOf<Self>>;

		/// Time used for validating register cert
		type UnixTime: UnixTime;

		/// The basic amount of funds that slashed when node is offline or misbehavier
		#[pallet::constant]
		type SlashBalance: Get<BalanceOf<Self>>;

		/// Number of blocks that node's need report its work
		#[pallet::constant]
		type RoundDuration: Get<BlockNumberFor<Self>>;

		/// Number of rounds that file order is expired and need to renew or close
		#[pallet::constant]
		type FileOrderRounds: Get<u32>;

		/// The maximum number of replicas order included
		#[pallet::constant]
		type MaxFileReplicas: Get<u32>;

		/// The number of replicas will be rewarded
		#[pallet::constant]
		type EffectiveFileReplicas: Get<u32>;

		/// The maximum file size the network accepts
		#[pallet::constant]
		type MaxFileSize: Get<u64>;

		/// The maximum power of node
		#[pallet::constant]
		type MaxPower: Get<u64>;

		/// The maximum number of files in each report
		#[pallet::constant]
		type MaxReportFiles: Get<u32>;

		/// The basic amount of funds that must be spent when store an file to network.
		#[pallet::constant]
		type FileBaseFee: Get<BalanceOf<Self>>;

		/// The additional funds that must be spent for the number of bytes of the file
		#[pallet::constant]
		type FileBytePrice: Get<BalanceOf<Self>>;

		/// The ratio for divide store reward to node's have replicas and round store reward.
		#[pallet::constant]
		type StoreRewardRatio: Get<Perbill>;

		/// Number fo founds to stash for registering a node
		#[pallet::constant]
		type StashBalance: Get<BalanceOf<Self>>;

		/// Mine factor
		#[pallet::constant]
		type MineFactor: Get<Perbill>;

		/// The maximum number of deer the storage mine in each report round
		#[pallet::constant]
		type MaxMine: Get<BalanceOf<Self>>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	/// The Tee enclaves
	#[pallet::storage]
	pub type Enclaves<T: Config> = StorageMap<_, Twox64Concat, EnclaveId, BlockNumberFor<T>>;

	/// Information stashing
	#[pallet::storage]
	pub type Stashs<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, StashInfo<T::AccountId, BalanceOf<T>>>;

	/// Number of rounds that reserved to storage pot
	#[pallet::storage]
	pub type StoragePotReserved<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// Node information
	#[pallet::storage]
	pub type Nodes<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, NodeInfo<BlockNumberFor<T>>>;

	/// Node register information
	#[pallet::storage]
	pub type Registers<T: Config> = StorageMap<_, Twox64Concat, MachineId, RegisterInfo>;

	/// Record current round
	#[pallet::storage]
	pub type CurrentRound<T: Config> = StorageValue<_, RoundIndex, ValueQuery>;

	/// Record the block number next round starts
	#[pallet::storage]
	pub type NextRoundAt<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// Node stats in a round
	#[pallet::storage]
	pub type RoundsReport<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		RoundIndex,
		Blake2_128Concat,
		T::AccountId,
		NodeStats,
		OptionQuery,
	>;

	/// Network stats in a round
	#[pallet::storage]
	pub type RoundsSummary<T: Config> =
		StorageMap<_, Twox64Concat, RoundIndex, SummaryStats, ValueQuery>;

	/// Information round rewards
	#[pallet::storage]
	pub type RoundsReward<T: Config> =
		StorageMap<_, Twox64Concat, RoundIndex, RewardInfo<BalanceOf<T>>, ValueQuery>;

	/// Information for stored files
	#[pallet::storage]
	pub type StoreFiles<T: Config> =
		StorageMap<_, Twox64Concat, FileId, StoreFile<BalanceOf<T>, BlockNumberFor<T>>>;

	/// Information for file orders
	#[pallet::storage]
	pub type FileOrders<T: Config> = StorageMap<
		_,
		Twox64Concat,
		FileId,
		FileOrder<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Add or change enclave.
		SetEnclave { enclave_id: EnclaveId, expire_at: BlockNumberFor<T> },
		/// A account have been stashed.
		Stashed { node: T::AccountId },
		/// A account have withdrawn some founds.
		Withdrawn { node: T::AccountId, stasher: T::AccountId, amount: BalanceOf<T> },
		/// A node was registerd.
		NodeRegisted { node: T::AccountId, machine_id: MachineId },
		/// A node reported its work.
		NodeReported {
			node: T::AccountId,
			machine_id: MachineId,
			round: RoundIndex,
			slash: BalanceOf<T>,
			mine_reward: BalanceOf<T>,
			share_store_reward: BalanceOf<T>,
			direct_store_reward: BalanceOf<T>,
		},
		/// A request to store file was submitted.
		StoreFileSubmitted { cid: FileId, caller: T::AccountId, fee: BalanceOf<T>, first: bool },
		/// A file have been removed.
		StoreFileRemoved { cid: FileId },
		/// A file order was created or renewed
		StoreFileNewOrder { cid: FileId, replicas: u32 },
		/// A file was deleted by admin.
		FileForceDeleted { cid: FileId },
		/// A round was ended.
		RoundEnded { round: RoundIndex, unpaid: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Enclave's expire time should not great than current
		InvalidEnclaveExpire,
		/// Node have been stashed with another account
		InvalidStashPair,
		/// Node's deposit is not enough to withdraw
		NoEnoughToWithdraw,
		/// Have not stashed node
		UnstashNode,
		/// Machine id incorrect
		MismatchMacheId,
		/// Machine id exists in system
		MachineAlreadyRegistered,
		/// IAS signature incorrenct
		InvalidIASSign,
		/// IAS cert incorrenct
		InvalidIASSigningCert,
		/// IAS body incorrenct
		InvalidIASBody,
		/// Enclave id incorrenct
		InvalidEnclave,
		/// Already reported in current round
		DuplicateReport,
		/// Fail to verify signature
		InvalidVerifyP256Sig,
		/// Report files or power exceed limit
		ReportExceedLimit,
		/// Node is unregisterd
		UnregisterNode,
		/// Not enough fee
		NotEnoughFee,
		/// File size incorrenct
		InvalidFileSize,
		/// Unable to delete file
		UnableToDeleteFile,
		/// Insufficient stash
		InsufficientStash,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> frame_support::weights::Weight {
			let next_round_at = NextRoundAt::<T>::get();
			if now >= next_round_at {
				Self::on_round_end();
			}
			// TODO: weights
			0
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub enclaves: Vec<(EnclaveId, BlockNumberFor<T>)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { enclaves: Default::default() }
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			<Pallet<T>>::next_round();
			let storage_pot = <Pallet<T>>::account_id();
			let min = T::Currency::minimum_balance();
			if T::Currency::free_balance(&storage_pot) < min {
				let _ = T::Currency::make_free_balance_be(&storage_pot, min);
			}
			for (code, bn) in &self.enclaves {
				Enclaves::<T>::insert(code.clone(), bn);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add or change expire of TEE enclave
		#[pallet::weight((T::WeightInfo::set_enclave(), DispatchClass::Operational))]
		pub fn set_enclave(
			origin: OriginFor<T>,
			enclave_id: EnclaveId,
			expire_at: T::BlockNumber,
		) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(old_expire_at) = Enclaves::<T>::get(&enclave_id) {
				ensure!(expire_at < old_expire_at, Error::<T>::InvalidEnclaveExpire);
			}
			Enclaves::<T>::insert(&enclave_id, &expire_at);
			Self::deposit_event(Event::<T>::SetEnclave { enclave_id, expire_at });

			Ok(())
		}

		/// Stash a account so it can be used for a storage node, the amount of funds to stash is
		/// T::StashBalance
		#[pallet::weight(1_000_000)]
		pub fn stash(
			origin: OriginFor<T>,
			node: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			let stasher = ensure_signed(origin)?;
			let node = T::Lookup::lookup(node)?;
			let stash_balance = T::StashBalance::get();
			if let Some(mut stash_info) = Stashs::<T>::get(&node) {
				ensure!(&stash_info.stasher == &stasher, Error::<T>::InvalidStashPair);
				if stash_info.deposit < stash_balance {
					let lack = stash_balance.saturating_sub(stash_info.deposit);
					T::Currency::transfer(
						&stasher,
						&Self::account_id(),
						lack,
						ExistenceRequirement::KeepAlive,
					)?;
					stash_info.deposit = stash_balance;
					Stashs::<T>::insert(node, stash_info);
				}
			} else {
				T::Currency::transfer(
					&stasher,
					&Self::account_id(),
					stash_balance,
					ExistenceRequirement::KeepAlive,
				)?;
				Stashs::<T>::insert(
					&node,
					StashInfo { stasher, deposit: stash_balance, machine_id: None },
				);
				Self::deposit_event(Event::<T>::Stashed { node });
			}
			Ok(())
		}

		/// Withdraw the mine reward, node's despoist should not below T::StashBalance
		#[pallet::weight(T::WeightInfo::withdraw())]
		pub fn withdraw(origin: OriginFor<T>) -> DispatchResult {
			let node = ensure_signed(origin)?;
			let mut stash_info = Stashs::<T>::get(&node).ok_or(Error::<T>::UnstashNode)?;
			let stash_deposit: BalanceOf<T> = stash_info.deposit;
			let stash_balance = T::StashBalance::get();
			let amount = stash_deposit.saturating_sub(stash_balance);
			ensure!(!amount.is_zero(), Error::<T>::NoEnoughToWithdraw);
			stash_info.deposit = stash_balance;
			let stasher = stash_info.stasher.clone();
			T::Currency::transfer(
				&Self::account_id(),
				&stasher,
				amount,
				ExistenceRequirement::KeepAlive,
			)?;
			Stashs::<T>::insert(node.clone(), stash_info);
			Self::deposit_event(Event::<T>::Withdrawn { node, stasher, amount });
			Ok(())
		}

		/// Register a node
		#[pallet::weight((T::WeightInfo::register(), DispatchClass::Operational))]
		pub fn register(
			origin: OriginFor<T>,
			machine_id: MachineId,
			ias_cert: Vec<u8>,
			ias_sig: Vec<u8>,
			ias_body: Vec<u8>,
			sig: Vec<u8>,
		) -> DispatchResult {
			let node = ensure_signed(origin)?;
			let maybe_register_info = Registers::<T>::get(&machine_id);
			let mut stash_info = Stashs::<T>::get(&node).ok_or(Error::<T>::UnstashNode)?;
			if maybe_register_info.is_some() {
				ensure!(&stash_info.machine_id.is_some(), Error::<T>::MachineAlreadyRegistered);
			}
			if let Some(stash_machine_id) = &stash_info.machine_id {
				ensure!(stash_machine_id == &machine_id, Error::<T>::MismatchMacheId);
			}
			let dec_cert = base64::decode_config(&ias_cert, base64::STANDARD)
				.map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let sig_cert = webpki::EndEntityCert::from(&dec_cert)
				.map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let chain: Vec<&[u8]> = Vec::new();
			#[cfg(not(feature = "runtime-benchmarks"))]
			let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
			#[cfg(feature = "runtime-benchmarks")]
			let now: u64 = 1627833600;
			let time_now = webpki::Time::from_seconds_since_unix_epoch(now);
			sig_cert
				.verify_is_valid_tls_server_cert(
					SUPPORTED_SIG_ALGS,
					&IAS_SERVER_ROOTS,
					&chain,
					time_now,
				)
				.map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let dec_sig = base64::decode(&ias_sig).map_err(|_| Error::<T>::InvalidIASSign)?;
			sig_cert
				.verify_signature(&webpki::RSA_PKCS1_2048_8192_SHA256, &ias_body, &dec_sig)
				.map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let json_body: serde_json::Value =
				serde_json::from_slice(&ias_body).map_err(|_| Error::<T>::InvalidIASBody)?;
			let isv_quote_body = json_body
				.get("isvEnclaveQuoteBody")
				.and_then(|v| v.as_str())
				.ok_or(Error::<T>::InvalidIASBody)?;
			let isv_quote_body =
				base64::decode(isv_quote_body).map_err(|_| Error::<T>::InvalidIASBody)?;
			let now_at = Self::now_bn();
			let enclave = &isv_quote_body[112..144].to_vec();
			ensure!(
				<Enclaves<T>>::iter()
					.find(|(id, bn)| { bn > &now_at && id == enclave })
					.is_some(),
				Error::<T>::InvalidEnclave
			);
			let key = &isv_quote_body[368..].to_vec();
			let data: Vec<u8> =
				[&ias_cert[..], &ias_sig[..], &ias_body[..], &machine_id[..]].concat();
			ensure!(verify_p256_sig(&key, &data, &sig), Error::<T>::InvalidVerifyP256Sig);

			match Registers::<T>::get(&machine_id) {
				Some(mut register) => {
					register.key = key.clone();
					register.enclave = enclave.clone();
					Registers::<T>::insert(&machine_id, register);
				},
				None => {
					Registers::<T>::insert(
						&machine_id,
						RegisterInfo { key: key.clone(), enclave: enclave.clone() },
					);
					stash_info.machine_id = Some(machine_id.clone());
					Stashs::<T>::insert(&node, stash_info);
				},
			}

			Self::deposit_event(Event::<T>::NodeRegisted { node, machine_id });
			Ok(())
		}

		/// Report storage work.
		#[pallet::weight((
			T::WeightInfo::report(add_files.len() as u32, del_files.len() as u32),
			DispatchClass::Operational
		))]
		pub fn report(
			origin: OriginFor<T>,
			#[pallet::compact] rid: u64,
			#[pallet::compact] power: u64,
			sig: Vec<u8>,
			add_files: Vec<(FileId, u64)>,
			del_files: Vec<FileId>,
			settle_files: Vec<FileId>,
		) -> DispatchResult {
			let reporter = ensure_signed(origin)?;
			ensure!(
				add_files.len() <= T::MaxReportFiles::get() as usize ||
					settle_files.len() <= T::MaxReportFiles::get() as usize,
				Error::<T>::ReportExceedLimit
			);
			let mut stash_info = Stashs::<T>::get(&reporter).ok_or(Error::<T>::UnstashNode)?;
			let machine_id =
				stash_info.machine_id.as_ref().ok_or(Error::<T>::UnregisterNode)?.clone();

			ensure!(stash_info.deposit >= T::SlashBalance::get(), Error::<T>::InsufficientStash);

			let register = Registers::<T>::get(&machine_id).ok_or(Error::<T>::UnregisterNode)?;
			let now_at = Self::now_bn();
			let enclave_bn =
				Enclaves::<T>::get(&register.enclave).ok_or(Error::<T>::InvalidEnclave)?;
			ensure!(now_at <= enclave_bn, Error::<T>::InvalidEnclave);

			let current_round = CurrentRound::<T>::get();
			let prev_round = current_round.saturating_sub(One::one());
			let maybe_node_info = Nodes::<T>::get(&reporter);
			if let Some(_) = &maybe_node_info {
				ensure!(
					!RoundsReport::<T>::contains_key(current_round, &reporter),
					Error::<T>::DuplicateReport
				);
			}
			let mut node_info = maybe_node_info.unwrap_or_default();
			let data: Vec<u8> = [
				&machine_id[..],
				&register.key[..],
				&encode_u64(node_info.rid)[..],
				&encode_u64(rid)[..],
				&encode_u64(power)[..],
				&encode_add_files(&add_files)[..],
				&encode_del_files(&del_files)[..],
			]
			.concat();
			ensure!(verify_p256_sig(&register.key, &data, &sig), Error::<T>::InvalidVerifyP256Sig);

			let mut replica_changes: Vec<(T::AccountId, u64, bool)> = vec![];
			let mut current_round_reward = RoundsReward::<T>::get(current_round);
			let mut storage_pot_reserved = StoragePotReserved::<T>::get();
			let mut node_inc_deposits: BTreeMap<T::AccountId, BalanceOf<T>> = BTreeMap::new();
			let mut nodes_prev_reported: BTreeMap<T::AccountId, bool> = BTreeMap::new();

			for cid in settle_files.iter() {
				Self::settle_file(
					&mut replica_changes,
					&mut current_round_reward.store_reward,
					&mut storage_pot_reserved,
					&mut stash_info.deposit,
					&mut node_inc_deposits,
					&mut nodes_prev_reported,
					&reporter,
					cid,
					prev_round,
				);
			}

			for (cid, file_size, ..) in add_files.iter() {
				if file_size > &T::MaxFileSize::get() {
					continue
				}
				Self::add_file(
					&mut replica_changes,
					&mut current_round_reward.store_reward,
					&mut storage_pot_reserved,
					&mut nodes_prev_reported,
					&mut stash_info.deposit,
					&reporter,
					cid,
					prev_round,
					*file_size,
				);
			}
			for cid in del_files.iter() {
				Self::delete_file(&mut replica_changes, &reporter, cid);
			}

			let mut share_store_reward = Zero::zero();
			let mut mine_reward = Zero::zero();
			let mut slash = Zero::zero();

			if let Some(stats) = RoundsReport::<T>::get(prev_round, &reporter) {
				Self::reward_round(
					prev_round,
					stats,
					&mut stash_info,
					&mut share_store_reward,
					&mut mine_reward,
				);
			} else {
				if !node_info.reported_at.is_zero() {
					Self::slash_offline(
						&mut storage_pot_reserved,
						&mut stash_info.deposit,
						&mut slash,
					);
				}
			}

			let mut node_changes: BTreeMap<T::AccountId, (u64, u64)> = BTreeMap::new();
			for (account, file_size, is_add) in replica_changes.iter() {
				let (size_add, size_sub) = node_changes.entry(account.clone()).or_default();
				match *is_add {
					true => *size_add = size_add.saturating_add(*file_size),
					false => *size_sub = size_sub.saturating_add(*file_size),
				}
			}

			for (account, (size_inc, size_dec)) in node_changes.iter() {
				if account == &reporter {
					node_info.used =
						node_info.used.saturating_add(*size_inc).saturating_sub(*size_dec);
				} else {
					Nodes::<T>::mutate(account, |maybe_node| {
						if let Some(other_node) = maybe_node {
							other_node.used =
								other_node.used.saturating_add(*size_inc).saturating_sub(*size_dec);
						}
					})
				}
			}
			node_info.power = power.min(T::MaxPower::get());

			let mut direct_store_reward: BalanceOf<T> = Zero::zero();
			for (account, inc) in node_inc_deposits.iter() {
				Stashs::<T>::mutate(account, |maybe_stash_info| {
					if let Some(stash_info) = maybe_stash_info {
						stash_info.deposit = stash_info.deposit.saturating_add(*inc);
						direct_store_reward = direct_store_reward.saturating_add(*inc);
					} else {
						current_round_reward.store_reward =
							current_round_reward.store_reward.saturating_add(*inc);
					}
				})
			}

			node_info.rid = rid;
			node_info.reported_at = now_at;

			StoragePotReserved::<T>::mutate(|v| *v = storage_pot_reserved);
			RoundsReward::<T>::insert(current_round, current_round_reward);
			RoundsReport::<T>::insert(
				current_round,
				reporter.clone(),
				NodeStats { power: node_info.power, used: node_info.used },
			);
			RoundsSummary::<T>::mutate(current_round, |v| {
				v.used = v.used.saturating_add(node_info.used.saturated_into());
				v.power = v.power.saturating_add(node_info.power.saturated_into());
			});
			Nodes::<T>::insert(reporter.clone(), node_info);
			Stashs::<T>::insert(reporter.clone(), stash_info);
			Self::deposit_event(Event::<T>::NodeReported {
				node: reporter,
				machine_id,
				round: current_round,
				slash,
				mine_reward,
				share_store_reward,
				direct_store_reward,
			});
			Ok(())
		}

		/// Add file to storage
		#[pallet::weight(T::WeightInfo::store())]
		pub fn store(
			origin: OriginFor<T>,
			cid: FileId,
			file_size: u64,
			fee: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				file_size > 0 && file_size <= T::MaxFileSize::get(),
				Error::<T>::InvalidFileSize
			);

			if let Some(mut file) = StoreFiles::<T>::get(&cid) {
				let new_reserved = fee.saturating_add(file.reserved);
				let min_fee = Self::store_file_bytes_fee(file.file_size);
				ensure!(new_reserved >= min_fee, Error::<T>::NotEnoughFee);
				T::Currency::transfer(
					&who,
					&Self::account_id(),
					fee,
					ExistenceRequirement::KeepAlive,
				)?;
				file.reserved = new_reserved;
				StoreFiles::<T>::insert(cid.clone(), file);
				Self::deposit_event(Event::<T>::StoreFileSubmitted {
					cid,
					caller: who,
					fee,
					first: false,
				});
			} else {
				let min_fee = Self::store_file_fee(file_size);
				ensure!(fee >= min_fee, Error::<T>::NotEnoughFee);
				T::Currency::transfer(
					&who,
					&Self::account_id(),
					fee,
					ExistenceRequirement::KeepAlive,
				)?;
				let base_fee = T::FileBaseFee::get();
				StoreFiles::<T>::insert(
					cid.clone(),
					StoreFile {
						reserved: fee.saturating_sub(base_fee),
						base_fee,
						file_size,
						added_at: Self::now_bn(),
					},
				);
				Self::deposit_event(Event::<T>::StoreFileSubmitted {
					cid,
					caller: who,
					fee,
					first: true,
				});
			}
			Ok(())
		}

		/// Force delete unsoloved file by root
		#[pallet::weight(T::WeightInfo::force_delete())]
		pub fn force_delete(origin: OriginFor<T>, cid: FileId) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(file) = StoreFiles::<T>::get(&cid) {
				let now = Self::now_bn();
				let rounds = T::FileOrderRounds::get();
				let invalid_at = file.added_at.saturating_add(
					T::RoundDuration::get().saturating_mul(rounds.saturated_into()),
				);
				sp_std::if_std! {
					println!("base_fee={:?}, invalid_at={:?}, now={:?}", file.base_fee, invalid_at, now);
				}
				ensure!(
					!file.base_fee.is_zero() &&
						now > invalid_at && FileOrders::<T>::get(&cid).is_none(),
					Error::<T>::UnableToDeleteFile
				);
				StoragePotReserved::<T>::mutate(|v| {
					*v = v.saturating_add(file.base_fee).saturating_add(file.reserved)
				});
				StoreFiles::<T>::remove(&cid);
				Self::deposit_event(Event::<T>::FileForceDeleted { cid });
			}
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	fn on_round_end() {
		let current_round = CurrentRound::<T>::get();
		let mine_reward = Self::calculate_mine_reward(current_round);
		if !mine_reward.is_zero() {
			let storage_pot_reserved = StoragePotReserved::<T>::get();
			let (new_storage_pot_reserved, need_mine_reward) = if storage_pot_reserved > mine_reward
			{
				(storage_pot_reserved.saturating_sub(mine_reward), Zero::zero())
			} else {
				(Zero::zero(), mine_reward.saturating_sub(storage_pot_reserved))
			};
			if !need_mine_reward.is_zero() {
				T::Currency::deposit_creating(&Self::account_id(), need_mine_reward);
			}
			if new_storage_pot_reserved != storage_pot_reserved {
				StoragePotReserved::<T>::mutate(|v| *v = new_storage_pot_reserved);
			}
			RoundsReward::<T>::mutate(current_round, |reward| {
				reward.mine_reward = reward.mine_reward.saturating_add(mine_reward);
			});
		}

		let prev_round = current_round.saturating_sub(1);
		let mut unpaid_reward = Zero::zero();
		if !prev_round.is_zero() {
			let prev_reward = RoundsReward::<T>::get(prev_round);
			unpaid_reward = prev_reward
				.store_reward
				.saturating_add(prev_reward.mine_reward)
				.saturating_sub(prev_reward.paid_mine_reward)
				.saturating_sub(prev_reward.paid_store_reward);
			if !unpaid_reward.is_zero() {
				match T::Currency::withdraw(
					&Self::account_id(),
					unpaid_reward,
					WithdrawReasons::TRANSFER,
					ExistenceRequirement::AllowDeath,
				) {
					Ok(treasury) => {
						T::Treasury::on_unbalanced(treasury);
					},
					Err(e) => {
						log!(error, "Storage pot is lack of funds {:?}", e);
					},
				}
			}
		}

		Self::next_round();
		Self::clear_round_information(prev_round.saturating_sub(1));
		Self::deposit_event(Event::<T>::RoundEnded { round: current_round, unpaid: unpaid_reward });
	}

	fn next_round() {
		NextRoundAt::<T>::mutate(|v| *v = v.saturating_add(T::RoundDuration::get()));
		CurrentRound::<T>::mutate(|v| *v = v.saturating_add(1));
	}

	fn clear_round_information(round: RoundIndex) {
		if round.is_zero() {
			return
		}
		RoundsReport::<T>::remove_prefix(round, None);
		RoundsSummary::<T>::remove(round);
		RoundsReward::<T>::remove(round);
	}

	fn add_file(
		replica_changes: &mut Vec<(T::AccountId, u64, bool)>,
		current_round_store_reward: &mut BalanceOf<T>,
		storage_pot_reserved: &mut BalanceOf<T>,
		nodes_prev_reported: &mut BTreeMap<T::AccountId, bool>,
		reporter_despoit: &mut BalanceOf<T>,
		reporter: &T::AccountId,
		cid: &FileId,
		prev_round: RoundIndex,
		file_size: u64,
	) {
		if let Some(mut file_order) = FileOrders::<T>::get(cid) {
			let mut new_nodes = vec![];
			let mut exist = false;
			for node in file_order.replicas.iter() {
				let reported = nodes_prev_reported
					.entry(node.clone())
					.or_insert_with(|| Self::round_reported(prev_round, node));
				if *reported {
					new_nodes.push(node.clone());
				} else {
					replica_changes.push((node.clone(), file_size, false));
				}
				if node == reporter {
					exist = true;
				}
			}
			if !exist && (new_nodes.len() as u32) < T::MaxFileReplicas::get() {
				new_nodes.push(reporter.clone());
				replica_changes.push((reporter.clone(), file_size, true));
			}
			file_order.replicas = new_nodes;
			FileOrders::<T>::insert(cid, file_order);
		} else {
			let ok = Self::settle_file_order(
				current_round_store_reward,
				storage_pot_reserved,
				reporter_despoit,
				cid,
				vec![reporter.clone()],
				Some(file_size),
			);
			if ok {
				replica_changes.push((reporter.clone(), file_size, true));
			}
		}
	}

	fn delete_file(
		replica_changes: &mut Vec<(T::AccountId, u64, bool)>,
		reporter: &T::AccountId,
		cid: &FileId,
	) {
		if let Some(mut file_order) = FileOrders::<T>::get(cid) {
			if let Ok(idx) = file_order.replicas.binary_search(reporter) {
				file_order.replicas.remove(idx);
				replica_changes.push((reporter.clone(), file_order.file_size, false));
				FileOrders::<T>::insert(cid, file_order);
			}
		}
	}

	fn settle_file(
		replica_changes: &mut Vec<(T::AccountId, u64, bool)>,
		current_round_store_reward: &mut BalanceOf<T>,
		storage_pot_reserved: &mut BalanceOf<T>,
		reporter_deposit: &mut BalanceOf<T>,
		node_inc_deposits: &mut BTreeMap<T::AccountId, BalanceOf<T>>,
		nodes_prev_reported: &mut BTreeMap<T::AccountId, bool>,
		reporter: &T::AccountId,
		cid: &FileId,
		prev_round: RoundIndex,
	) {
		if let Some(file_order) = FileOrders::<T>::get(cid) {
			if file_order.expire_at >= Self::now_bn() {
				return
			}

			let file_order_fee = file_order.fee;
			let mut total_order_reward: BalanceOf<T> = Zero::zero();
			let each_order_reward = Perbill::from_rational(1, T::EffectiveFileReplicas::get()) *
				T::StoreRewardRatio::get() *
				file_order_fee;
			let mut replicas = vec![];
			for node in file_order.replicas.iter() {
				let reported = nodes_prev_reported
					.entry(node.clone())
					.or_insert_with(|| Self::round_reported(prev_round, node));
				if *reported {
					if node == reporter {
						*reporter_deposit = reporter_deposit.saturating_add(each_order_reward);
					} else {
						let node_deposit = node_inc_deposits.entry(node.clone()).or_default();
						*node_deposit = node_deposit.saturating_add(each_order_reward);
					}

					total_order_reward = total_order_reward.saturating_add(each_order_reward);
					replicas.push(node.clone());
				} else {
					replica_changes.push((node.clone(), file_order.file_size, false));
				}
			}
			let ok = Self::settle_file_order(
				current_round_store_reward,
				storage_pot_reserved,
				reporter_deposit,
				cid,
				replicas.clone(),
				None,
			);
			if !ok {
				for node in replicas.iter() {
					replica_changes.push((node.clone(), file_order.file_size, false));
				}
			}
			let unpaid_reward = file_order_fee.saturating_sub(total_order_reward);
			if !unpaid_reward.is_zero() {
				*current_round_store_reward =
					current_round_store_reward.saturating_add(unpaid_reward);
			}
		}
	}

	fn settle_file_order(
		current_round_store_reward: &mut BalanceOf<T>,
		storage_pot_reserved: &mut BalanceOf<T>,
		reporter_despoit: &mut BalanceOf<T>,
		cid: &FileId,
		nodes: Vec<T::AccountId>,
		maybe_file_size: Option<u64>,
	) -> bool {
		if let Some(mut file) = StoreFiles::<T>::get(cid) {
			let expect_order_fee =
				Self::store_file_bytes_fee(maybe_file_size.unwrap_or(file.file_size));
			if let Some(file_size) = maybe_file_size {
				if !file.base_fee.is_zero() {
					*storage_pot_reserved = storage_pot_reserved.saturating_add(file.base_fee);
					// user underreported the file size
					if file.file_size < file_size && file.reserved < expect_order_fee {
						let to_reporter_reward =
							Perbill::from_rational(1, T::EffectiveFileReplicas::get()) *
								T::StoreRewardRatio::get() * file.reserved;
						*reporter_despoit = reporter_despoit.saturating_add(to_reporter_reward);
						*current_round_store_reward = current_round_store_reward
							.saturating_add(file.reserved.saturating_sub(to_reporter_reward));
						Self::clear_store_file(cid);
						return false
					}
					file.base_fee = Zero::zero();
					file.file_size = file_size;
				}
			}
			let (mut order_fee, new_reserved) = if file.reserved > expect_order_fee {
				(expect_order_fee, file.reserved.saturating_sub(expect_order_fee))
			} else {
				(file.reserved, Zero::zero())
			};
			if order_fee < expect_order_fee {
				*storage_pot_reserved = storage_pot_reserved.saturating_add(order_fee);
				order_fee = Zero::zero();
			}
			if order_fee.is_zero() {
				Self::clear_store_file(cid);
				return false
			}
			Self::deposit_event(Event::<T>::StoreFileNewOrder {
				cid: cid.clone(),
				replicas: nodes.len() as u32,
			});
			FileOrders::<T>::insert(
				cid,
				FileOrder {
					fee: order_fee,
					file_size: file.file_size,
					expire_at: Self::get_file_order_expire(),
					replicas: nodes,
				},
			);
			file.reserved = new_reserved;
			StoreFiles::<T>::insert(cid, file);
			true
		} else {
			false
		}
	}

	fn reward_round(
		prev_round: RoundIndex,
		node_stats: NodeStats,
		stash_info: &mut StashInfo<T::AccountId, BalanceOf<T>>,
		report_store_reward: &mut BalanceOf<T>,
		report_mine_reward: &mut BalanceOf<T>,
	) {
		if prev_round.is_zero() {
			return
		}
		RoundsReward::<T>::mutate(prev_round, |mut reward_info| {
			let summary = RoundsSummary::<T>::get(prev_round);
			let used_ratio = Perbill::from_rational(node_stats.used as u128, summary.used);
			let power_ratio = Perbill::from_rational(node_stats.power as u128, summary.power);
			let store_reward = used_ratio * reward_info.store_reward;
			let mine_reward = power_ratio * reward_info.mine_reward;
			*report_store_reward = report_store_reward.saturating_add(store_reward);
			*report_mine_reward = report_mine_reward.saturating_add(mine_reward);
			reward_info.paid_store_reward =
				reward_info.paid_store_reward.saturating_add(store_reward);
			reward_info.paid_mine_reward = reward_info.paid_mine_reward.saturating_add(mine_reward);
			stash_info.deposit =
				stash_info.deposit.saturating_add(mine_reward).saturating_add(store_reward);
		});
	}

	fn slash_offline(
		storage_pot_reserved: &mut BalanceOf<T>,
		reporter_deposit: &mut BalanceOf<T>,
		slash: &mut BalanceOf<T>,
	) {
		let slash_balance = T::SlashBalance::get();
		if slash_balance.is_zero() {
			return
		}
		let (slash_reserved, new_deposit) = if *reporter_deposit > slash_balance {
			(slash_balance, reporter_deposit.saturating_sub(slash_balance))
		} else {
			(*reporter_deposit, Zero::zero())
		};
		*slash = slash.saturating_add(slash_reserved);
		*reporter_deposit = new_deposit;
		*storage_pot_reserved = storage_pot_reserved.saturating_add(slash_reserved);
	}

	fn round_reported(round: RoundIndex, node: &T::AccountId) -> bool {
		if round.is_zero() {
			return true
		}
		if RoundsReport::<T>::contains_key(round, node) {
			return true
		}
		Nodes::<T>::get(&node).map(|v| v.reported_at.is_zero()).unwrap_or_default()
	}

	fn calculate_mine_reward(round: RoundIndex) -> BalanceOf<T> {
		let summary = RoundsSummary::<T>::get(round);
		if summary.power.is_zero() {
			return Zero::zero()
		}

		let mine_reward: BalanceOf<T> = (T::MineFactor::get() * summary.power).saturated_into();
		mine_reward.min(T::MaxMine::get())
	}

	fn clear_store_file(cid: &FileId) {
		StoreFiles::<T>::remove(cid);
		FileOrders::<T>::remove(cid);
		Self::deposit_event(Event::<T>::StoreFileRemoved { cid: cid.clone() });
	}

	fn store_file_fee(file_size: u64) -> BalanceOf<T> {
		T::FileBaseFee::get().saturating_add(Self::store_file_bytes_fee(file_size))
	}

	fn store_file_bytes_fee(file_size: u64) -> BalanceOf<T> {
		let mut file_size_in_mega = file_size / 1_048_576;
		if file_size % 1_048_576 != 0 {
			file_size_in_mega += 1;
		}
		T::FileBytePrice::get().saturating_mul(file_size_in_mega.saturated_into())
	}

	fn get_file_order_expire() -> BlockNumberFor<T> {
		let now_at = Self::now_bn();
		let rounds = T::FileOrderRounds::get();
		now_at.saturating_add(T::RoundDuration::get().saturating_mul(rounds.saturated_into()))
	}

	fn now_bn() -> BlockNumberFor<T> {
		<frame_system::Pallet<T>>::block_number()
	}
}

pub fn verify_p256_sig(pk: &Vec<u8>, data: &Vec<u8>, sig: &Vec<u8>) -> bool {
	let mut pk = pk.clone();
	let mut sig = sig.clone();

	pk[0..32].reverse();
	pk[32..].reverse();

	sig[0..32].reverse();
	sig[32..].reverse();

	let vk: Vec<u8> = [&vec![4][..], &pk[..]].concat();

	if let (Ok(sig), Ok(vk)) = (Signature::from_bytes(&sig), VerifyingKey::from_sec1_bytes(&vk[..]))
	{
		return vk.verify(data, &sig).is_ok()
	}
	false
}

fn encode_u64(number: u64) -> Vec<u8> {
	let mut value = number;
	let mut encoded_number: Vec<u8> = [].to_vec();
	loop {
		encoded_number.push((value % 10) as u8 + 48u8); // "0" is 48u8
		value /= 10;
		if value == 0 {
			break
		}
	}
	encoded_number.reverse();
	encoded_number
}

fn encode_add_files(list: &Vec<(FileId, u64)>) -> Vec<u8> {
	let mut output = vec![];
	for (cid, size) in list.iter() {
		output.extend(cid.clone());
		output.extend(encode_u64(*size));
	}
	output
}

fn encode_del_files(list: &Vec<FileId>) -> Vec<u8> {
	let mut output = vec![];
	for cid in list.iter() {
		output.extend(cid.clone());
	}
	output
}
