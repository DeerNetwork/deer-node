//! # Storage Online Module

#![cfg_attr(not(feature = "std"), no_std)]


// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

mod constants;

pub use constants::*;

// pub mod weights;


use sp_std::{prelude::*, collections::btree_map::BTreeMap};
use sp_runtime::{Perbill, RuntimeDebug, SaturatedConversion, traits::{Zero, One, StaticLookup, Saturating, AccountIdConversion}};
use codec::{Encode, Decode};
use frame_support::{
	traits::{Currency, ReservableCurrency, ExistenceRequirement, UnixTime, Get},
};
use frame_system::{Config as SystemConfig, pallet_prelude::BlockNumberFor};
use p256::ecdsa::{VerifyingKey, signature::{Verifier, Signature}};

pub type FileId = Vec<u8>;
pub type EnclaveId = Vec<u8>;
pub type PubKey = Vec<u8>;
pub type MachineId = Vec<u8>;
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;
pub type RoundIndex = u32;

// pub use weights::WeightInfo;
pub use pallet::*;

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

pub trait RoundPayout<Balance> {
	fn round_payout(total_size: u128) -> Balance;
}

impl<Balance: Default> RoundPayout<Balance> for () {
	fn round_payout(_total_size: u128) -> Balance {
		Default::default()
	}
}

/// Node information
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default)]
pub struct NodeInfo {
	/// A increment id of one report
    pub rid: u64,
	/// Effective storage space
	pub used: u64,
	/// Mine power of node, use this to distribute mining rewards 
	pub power: u64,
	/// The lastest round node reported itself
	pub last_round: RoundIndex,
}

/// Information round rewards
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default)]
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
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
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
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
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
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct StashInfo<AccountId, Balance> {
	/// Stasher account
    pub stasher: AccountId,
	/// Stash funds 
    pub deposit: Balance,
	/// Node's machine id
	pub machine_id: Option<MachineId>,
}

/// Information for TEE node
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct RegisterInfo {
	/// PUb key to verify signed message
	pub key: PubKey,
	/// Tee enclave id
	pub enclave: EnclaveId,
}

/// Record node's effictive storage size and power
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default)]
pub struct NodeStats {
	/// Node's power
	pub power: u64,
	/// Eeffictive storage size
	pub used: u64,
}

/// Record network's effictive storage size and power
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default)]
pub struct SummaryStats {
	/// Network's power
	pub power: u128,
	/// Eeffictive storage size
	pub used: u128,
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// Time used for validating register cert
		type UnixTime: UnixTime;

		/// The payout for mining in the current round.
		type RoundPayout: RoundPayout<BalanceOf<Self>>;

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

		/// The maximum file size the network accepts
		#[pallet::constant]
		type MaxFileSize: Get<u64>;

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

		/// Number of rounds to keep in history.
		#[pallet::constant]
        type HistoryRoundDepth: Get<u32>;
	}

	/// The Tee enclaves
	#[pallet::storage]
	pub type Enclaves<T: Config> = StorageMap<
		_,
		Twox64Concat,
		EnclaveId,
		BlockNumberFor<T>,
	>;

	/// Number of rounds that reserved to storage pot
	#[pallet::storage]
	pub type StoragePotReserved<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// Node information
	#[pallet::storage]
	pub type Nodes<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		NodeInfo,
	>;

	/// Node register information
	#[pallet::storage]
	pub type Registers<T: Config> = StorageMap<
		_,
		Twox64Concat,
		MachineId,
		RegisterInfo,
	>;

	/// Record current round
	#[pallet::storage]
	pub type CurrentRound<T: Config> = StorageValue<_, RoundIndex, ValueQuery>;

	/// Record network's effictive storage size and power
	#[pallet::storage]
	pub type Summary<T: Config> = StorageValue<_, SummaryStats, ValueQuery>;

	/// Record the block number when round end
	#[pallet::storage]
	pub type RoundsBlockNumber<T: Config> = StorageMap<
		_,
		Twox64Concat, RoundIndex,
		BlockNumberFor<T>,
        ValueQuery,
	>;

	/// Node stats in a round
	#[pallet::storage]
	pub type RoundsReport<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat, RoundIndex,
		Blake2_128Concat, T::AccountId,
		NodeStats, OptionQuery,
	>;

	/// Network stats in a round
	#[pallet::storage]
	pub type RoundsSummary<T: Config> = StorageMap<
		_,
		Twox64Concat, RoundIndex,
		SummaryStats, ValueQuery,
	>;

	/// Information for stored files
	#[pallet::storage]
	pub type StoreFiles<T: Config> = StorageMap<
		_,
		Twox64Concat, FileId,
		StoreFile<BalanceOf<T>, BlockNumberFor<T>>,
	>;

	/// Information for file orders
	#[pallet::storage]
	pub type FileOrders<T: Config> = StorageMap<
		_,
		Twox64Concat, FileId,
		FileOrder<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
	>;

	/// Information stashing 
	#[pallet::storage]
	pub type Stashs<T: Config> = StorageMap<
		_,
		Blake2_128Concat, T::AccountId,
		StashInfo<T::AccountId, BalanceOf<T>>,
	>;

	/// Information round rewards
	#[pallet::storage]
	pub type RoundsReward<T: Config> = StorageMap<
		_,
		Twox64Concat, RoundIndex,
		RewardInfo<BalanceOf<T>>, ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(
		T::AccountId = "AccountId",
		BalanceOf<T> = "Balance",
		MachineId = "MachineId",
		BlockNumberFor<T> = "BlockNumber",
	)]
	pub enum Event<T: Config> {
		/// Add or change enclave, \[enclave_id, expire_at\]
        SetEnclave(EnclaveId, BlockNumberFor<T>),
		/// A account have been stashed, \[node\]
        Stashed(T::AccountId),
		/// A node was registerd, \[node, machine_id\]
		NodeRegisted(T::AccountId, MachineId),
		/// A node reported its work, \[node, machine_id\]
		NodeReported(T::AccountId, MachineId),
		/// A account have withdrawn some founds, \[node, beneficary, amount\]
        Withdrawn(T::AccountId, T::AccountId, BalanceOf<T>),
		/// A file have summitted, \[file_id, account, fee\]
		StoreFileRequested(FileId, T::AccountId, BalanceOf<T>),
		/// More founds given to a file, \[file_id, account, fee\]
		StoreFileCharged(FileId, T::AccountId, BalanceOf<T>),
		/// A file have been removed, \[file_id\]
		StoreFileRemoved(FileId),
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
		/// Report files incorrect
		IllegalReportFiles,
		/// Node is unregisterd
		UnregisterNode,
		/// Not enough fee 
		NotEnoughFee,
		/// File size incorrenct
		InvalidFileSize,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> frame_support::weights::Weight {
            let next_round_bn = Self::get_next_round_bn();
			if now >= next_round_bn {
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
			GenesisConfig {
				enclaves: Default::default(),
			}
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			CurrentRound::<T>::mutate(|v| *v = 1);
			let storage_pot = <Pallet<T>>::storage_pot();
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
		#[pallet::weight((1_000_000, DispatchClass::Operational))]
		pub fn set_enclave(
			origin: OriginFor<T>,
			enclave: EnclaveId,
			expire: T::BlockNumber,
		) -> DispatchResult {
            ensure_root(origin)?;
            if let Some(old_expire) = Enclaves::<T>::get(&enclave) {
                ensure!(expire < old_expire, Error::<T>::InvalidEnclaveExpire);
            }
            Enclaves::<T>::insert(&enclave, &expire);
            Self::deposit_event(Event::<T>::SetEnclave(enclave, expire));

            Ok(())
		}

		/// Stash a account so it can be used for a storage node, the amount of funds to stash is T::StashBalance
		#[pallet::weight(1_000_000)]
		pub fn stash(
			origin: OriginFor<T>,
			node: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			let stasher = ensure_signed(origin)?;
			let controller = T::Lookup::lookup(node)?;
			let stash_balance = T::StashBalance::get();
			if let Some(mut stash_info) = Stashs::<T>::get(&controller) {
				ensure!(&stash_info.stasher == &stasher, Error::<T>::InvalidStashPair);
				if stash_info.deposit < stash_balance {
					let lack = stash_balance.saturating_sub(stash_info.deposit);
					T::Currency::transfer(&stasher, &Self::storage_pot(), lack, ExistenceRequirement::KeepAlive)?;
					stash_info.deposit = stash_balance;
					Stashs::<T>::insert(controller, stash_info);
				}
			} else {
				T::Currency::transfer(&stasher, &Self::storage_pot(), stash_balance, ExistenceRequirement::KeepAlive)?;
				Stashs::<T>::insert(&controller, StashInfo {
					stasher,
					deposit: stash_balance,
					machine_id: None,
				});
				Self::deposit_event(Event::<T>::Stashed(controller));
			}
			Ok(())
		}

		/// Withdraw the mine reward, node's despoist should not below T::StashBalance
		#[pallet::weight(1_000_000)]
		pub fn withdraw(
			origin: OriginFor<T>,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
            let mut stash_info = Stashs::<T>::get(&controller).ok_or(Error::<T>::UnstashNode)?;
            let stash_deposit: BalanceOf<T> = stash_info.deposit;
			let stash_balance = T::StashBalance::get();
			let profit = stash_deposit.saturating_sub(stash_balance);
			ensure!(!profit.is_zero(), Error::<T>::NoEnoughToWithdraw);
			stash_info.deposit = stash_balance;
			let stasher = stash_info.stasher.clone();
			T::Currency::transfer(&Self::storage_pot(), &stasher, profit, ExistenceRequirement::KeepAlive)?;
            Stashs::<T>::insert(controller.clone(), stash_info);
            Self::deposit_event(Event::<T>::Withdrawn(controller, stasher, profit));
            Ok(())
		}

		/// Register a node 
		#[pallet::weight((1_000_000, DispatchClass::Operational))]
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
			let dec_cert = base64::decode_config(&ias_cert, base64::STANDARD).map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let sig_cert = webpki::EndEntityCert::from(&dec_cert).map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let chain: Vec<&[u8]> = Vec::new();
			let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
			let time_now = webpki::Time::from_seconds_since_unix_epoch(now);
			sig_cert.verify_is_valid_tls_server_cert(
				SUPPORTED_SIG_ALGS,
				&IAS_SERVER_ROOTS,
				&chain,
				time_now
			).map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let dec_sig = base64::decode(&ias_sig).map_err(|_| Error::<T>::InvalidIASSign)?;
			sig_cert.verify_signature(
				&webpki::RSA_PKCS1_2048_8192_SHA256,
				&ias_body,
				&dec_sig
			).map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let json_body: serde_json::Value = serde_json::from_slice(&ias_body).map_err(|_| Error::<T>::InvalidIASBody)?;
			let isv_quote_body = json_body.get("isvEnclaveQuoteBody").and_then(|v| v.as_str()).ok_or(Error::<T>::InvalidIASBody)?;
			let isv_quote_body = base64::decode(isv_quote_body).map_err(|_| Error::<T>::InvalidIASBody)?;
			let now_at = Self::now_bn();
			let enclave = &isv_quote_body[112..144].to_vec();
			ensure!(<Enclaves<T>>::iter().find(|(id, bn)| { bn > &now_at && id ==  enclave }).is_some(), Error::<T>::InvalidEnclave);
			let key = &isv_quote_body[368..].to_vec();
			let data: Vec<u8> = [
				&ias_cert[..],
				&ias_sig[..],
				&ias_body[..],
				&machine_id[..],
			].concat();
			ensure!(verify_p256_sig(&key, &data, &sig), Error::<T>::InvalidVerifyP256Sig);

			match Registers::<T>::get(&machine_id) {
				Some(mut register) => {
					register.key = key.clone();
					register.enclave = enclave.clone();
					Registers::<T>::insert(&machine_id, register);
				},
				None => {
					Registers::<T>::insert(&machine_id, RegisterInfo {
						key: key.clone(),
						enclave: enclave.clone(),
					});
					stash_info.machine_id = Some(machine_id.clone());
					Stashs::<T>::insert(&node, stash_info);
				}
			}
			
			Self::deposit_event(Event::<T>::NodeRegisted(node, machine_id));
            Ok(())
		}

		/// Report storage work.
		#[pallet::weight(1_000_000)]
		pub fn report(
			origin: OriginFor<T>,
			machine_id: MachineId,
			rid: u64,
			sig: Vec<u8>,
			add_files: Vec<(FileId, u64)>,
			del_files: Vec<FileId>,
			settle_files: Vec<FileId>
		) -> DispatchResult {
			let reporter = ensure_signed(origin)?;
            ensure!(
				add_files.len() < T::MaxReportFiles::get() as usize ||
				del_files.len() < T::MaxReportFiles::get() as usize ||
				settle_files.len() < T::MaxReportFiles::get() as usize, 
				Error::<T>::IllegalReportFiles
			);
			let mut stash_info = Stashs::<T>::get(&reporter).ok_or(Error::<T>::UnstashNode)?;
			ensure!(stash_info.machine_id.is_some(), Error::<T>::UnregisterNode);
			ensure!(&stash_info.machine_id.clone().unwrap() == &machine_id , Error::<T>::MismatchMacheId);
			let register = Registers::<T>::get(&machine_id).ok_or(Error::<T>::UnregisterNode)?;
			let now_at = Self::now_bn();
			let enclave_bn = Enclaves::<T>::get(&register.enclave).ok_or(Error::<T>::InvalidEnclave)?;
			ensure!(now_at <= enclave_bn, Error::<T>::InvalidEnclave);

            let current_round = CurrentRound::<T>::get();
			let prev_round = current_round.saturating_sub(One::one());
			let maybe_node_info: Option<NodeInfo> = Nodes::<T>::get(&reporter);
			if let Some(_) = &maybe_node_info {
				ensure!(!RoundsReport::<T>::contains_key(current_round, &reporter), Error::<T>::DuplicateReport);
			}
			let mut node_info = maybe_node_info.unwrap_or_default();
			let data: Vec<u8> = [
				&machine_id[..],
				&register.key[..],
				&encode_u64(node_info.rid)[..],
				&encode_u64(rid)[..],
				&encode_add_files(&add_files)[..],
				&encode_del_files(&del_files)[..],
			].concat();
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
					prev_round
				);
			}

			for (cid, file_size, ..) in add_files.iter() {
				if file_size > &T::MaxFileSize::get() {
					continue;
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
					*file_size
				);
			}
			for cid in del_files.iter() {
				Self::delete_file(&mut replica_changes, &reporter, cid);
			}

			if let Some(stats) = RoundsReport::<T>::get(prev_round, &reporter) {
				Self::round_reward(prev_round, stats, &mut stash_info);
			} else {
				if !node_info.last_round.is_zero() {
					Self::slash_offline(
						&mut storage_pot_reserved,
						&mut stash_info.deposit
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

			let mut sumary = Summary::<T>::get();
			for (account, (size_inc, size_sub)) in node_changes.iter() {
				if account == &reporter {
					node_info.power = node_info.power.saturating_add(*size_inc);
					node_info.used = node_info.used.saturating_add(*size_inc).saturating_sub(*size_sub);
					sumary.power = sumary.power.saturating_add(*size_inc as u128);
					sumary.used = sumary.used.saturating_add(*size_inc as u128).saturating_sub(*size_sub as u128);
				} else {
					Nodes::<T>::mutate(account, |maybe_node| {
						if let Some(other_node) = maybe_node {
							other_node.power = other_node.power.saturating_add(*size_inc);
							other_node.used = other_node.used.saturating_add(*size_inc).saturating_sub(*size_sub);
							sumary.power = sumary.power.saturating_add(*size_inc as u128);
							sumary.used = sumary.used.saturating_add(*size_inc as u128).saturating_sub(*size_sub as u128);
						}
					})
				}
			}

			for (account, inc) in node_inc_deposits.iter() {
				Stashs::<T>::mutate(account, |maybe_stash_info| {
					if let Some(stash_info) = maybe_stash_info {
						stash_info.deposit = stash_info.deposit.saturating_add(*inc);
					} else {
						current_round_reward.store_reward = current_round_reward.store_reward.saturating_add(*inc);
					}
				})
			}

			node_info.rid = rid;
			node_info.last_round = current_round;

			StoragePotReserved::<T>::mutate(|v| *v = storage_pot_reserved);
			RoundsReward::<T>::insert(current_round, current_round_reward);
			RoundsReport::<T>::insert(current_round, reporter.clone(),  NodeStats { power: node_info.power, used: node_info.used });
			Summary::<T>::mutate(|v|  *v = sumary);
			Nodes::<T>::insert(reporter.clone(), node_info);
			Stashs::<T>::insert(reporter.clone(), stash_info);
			Self::deposit_event(Event::<T>::NodeReported(reporter, machine_id));
			Ok(())
		}

		/// Add file to storage
		#[pallet::weight(1_000_000)]
		pub fn store(
			origin: OriginFor<T>,
			cid: FileId,
			file_size: u64,
			fee: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(file_size > 0 && file_size <= T::MaxFileSize::get(), Error::<T>::InvalidFileSize);

			if let Some(mut file) = StoreFiles::<T>::get(&cid) {
				let new_reserved = fee.saturating_add(file.reserved);
				let min_fee = Self::store_file_bytes_fee(file.file_size);
				ensure!(new_reserved >= min_fee, Error::<T>::NotEnoughFee);
                T::Currency::transfer(&who, &Self::storage_pot(), fee, ExistenceRequirement::KeepAlive)?;
				file.reserved = new_reserved;
				StoreFiles::<T>::insert(cid.clone(), file);
				Self::deposit_event(Event::<T>::StoreFileCharged(cid, who, fee));
			} else {
				let min_fee = Self::store_file_fee(file_size);
				ensure!(fee >= min_fee, Error::<T>::NotEnoughFee);
                T::Currency::transfer(&who, &Self::storage_pot(), fee, ExistenceRequirement::KeepAlive)?;
				let base_fee = T::FileBaseFee::get();
				StoreFiles::<T>::insert(cid.clone(), StoreFile {
					reserved: fee.saturating_sub(base_fee),
					base_fee,
					file_size,
					added_at: Self::now_bn(), // TODO: file is invalid if no order for a long time
				});
				Self::deposit_event(Event::<T>::StoreFileRequested(cid, who, fee));
			}
			Ok(())
		}
	}
}


impl<T: Config> Pallet<T> {

	pub fn storage_pot() -> T::AccountId {
		PALLET_ID.into_sub_account("stor")
	}

	fn on_round_end() {
        let current_round = CurrentRound::<T>::get();
        let next_round =  current_round.saturating_add(1);
        let prev_round =  current_round.saturating_sub(1);

		let summary = Summary::<T>::get();
		let mine_reward = T::RoundPayout::round_payout(summary.power);
		if !mine_reward.is_zero() {
			T::Currency::deposit_creating(&Self::storage_pot(), mine_reward);
		}
		let mut store_reward = Zero::zero();
		if !prev_round.is_zero() {
			let prev_reward = RoundsReward::<T>::get(prev_round);
			store_reward = prev_reward.store_reward
				.saturating_add(prev_reward.mine_reward)
				.saturating_sub(prev_reward.paid_mine_reward)
				.saturating_sub(prev_reward.paid_store_reward);
		}
		RoundsReward::<T>::mutate(
			current_round, 
			|reward| {
				reward.mine_reward = reward.mine_reward.saturating_add(mine_reward);
				reward.store_reward = reward.store_reward.saturating_add(store_reward);
			}
		);

		RoundsSummary::<T>::insert(current_round, summary);
        RoundsBlockNumber::<T>::insert(next_round, Self::get_next_round_bn());
		CurrentRound::<T>::mutate(|v| *v = next_round);

        // let to_remove_round = current_round.saturating_sub(T::HistoryRoundDepth::get());
        // Self::clear_round_information(to_remove_round);
	}

    // fn clear_round_information(round: RoundIndex) {
	// 	if round.is_zero() { return; }
    //     RoundsReport::<T>::remove_prefix(round, None);
    //     RoundsBlockNumber::<T>::remove(round);
    //     RoundsSummary::<T>::remove(round);
    //     RoundsReward::<T>::remove(round);
    // }

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
				let reported = nodes_prev_reported.entry(node.clone()).or_insert_with(|| 
					Self::round_reported(prev_round, node)
				);
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
				Some(file_size)
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
				return;
			}

			let file_order_fee = file_order.fee;
			let mut total_order_reward: BalanceOf<T>  = Zero::zero();
			let each_order_reward = Perbill::from_rational(1, T::MaxFileReplicas::get()) * T::StoreRewardRatio::get() * file_order_fee;
			let mut replicas = vec![];
			for node in file_order.replicas.iter() {
				let reported = nodes_prev_reported.entry(node.clone()).or_insert_with(|| 
					Self::round_reported(prev_round, node)
				);
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
				None
			);
			if !ok {
				for node in replicas.iter() {
					replica_changes.push((node.clone(), file_order.file_size, false));
				}
			}
			let unpaid_reward = file_order_fee.saturating_sub(total_order_reward);
			if !unpaid_reward.is_zero() {
				*current_round_store_reward = current_round_store_reward.saturating_add(unpaid_reward);
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
			let expect_order_fee = Self::store_file_bytes_fee(maybe_file_size.unwrap_or(file.file_size));
			if let Some(file_size) = maybe_file_size {
				if !file.base_fee.is_zero() {
					*storage_pot_reserved =  storage_pot_reserved.saturating_add(file.base_fee);
					// user underreported the file size
					if file.file_size < file_size && file.reserved < expect_order_fee {
						let to_reporter_reward = Perbill::from_rational(1, T::MaxFileReplicas::get()) * T::StoreRewardRatio::get() * file.reserved;
						*reporter_despoit = reporter_despoit.saturating_add(to_reporter_reward);
						*current_round_store_reward = current_round_store_reward.saturating_add(file.reserved.saturating_sub(to_reporter_reward));
						Self::clear_store_file(cid);
						return false;
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
			if order_fee.is_zero() {
				Self::clear_store_file(cid);
				return false;
			}
			if order_fee < expect_order_fee {
				let lack_fee = expect_order_fee.saturating_sub(order_fee);
				if *storage_pot_reserved > lack_fee {
					order_fee = expect_order_fee;
					*storage_pot_reserved = storage_pot_reserved.saturating_sub(lack_fee);
				} else {
					order_fee = order_fee.saturating_add(*storage_pot_reserved);
					*storage_pot_reserved = Zero::zero();
				}
			}
			FileOrders::<T>::insert(cid, FileOrder {
				fee: order_fee,
				file_size: file.file_size,
				expire_at: Self::get_file_order_expire(),
				replicas: nodes,
			});
			file.reserved = new_reserved;
			StoreFiles::<T>::insert(cid, file);
			true
		} else {
			false
		}
	}

	fn round_reward(
		prev_round: RoundIndex,
		node_stats: NodeStats,
		stash_info: &mut StashInfo<T::AccountId, BalanceOf<T>>,
	) {
		if prev_round.is_zero() {
			return;
		}
		let mut reward_info =  RoundsReward::<T>::get(prev_round);
		let summary =  RoundsSummary::<T>::get(prev_round);
		let used_ratio = Perbill::from_rational(node_stats.used as u128, summary.used);
		let power_ratio = Perbill::from_rational(node_stats.power as u128, summary.power);
		let store_reward  = used_ratio * reward_info.store_reward;
		let mine_reward =  power_ratio * reward_info.mine_reward;
		reward_info.paid_store_reward = reward_info.paid_store_reward.saturating_add(store_reward);
		reward_info.paid_mine_reward = reward_info.paid_mine_reward.saturating_add(mine_reward);
		stash_info.deposit = stash_info.deposit.saturating_add(mine_reward).saturating_add(store_reward);
		RoundsReward::<T>::insert(prev_round, reward_info);
	}

	fn slash_offline(
		storage_pot_reserved: &mut BalanceOf<T>,
		reporter_deposit: &mut BalanceOf<T>,
	) {
		let slash_balance = T::SlashBalance::get();
		if slash_balance.is_zero() {
			return;
		}
		let (slash_reserved, new_deposit) = if *reporter_deposit > slash_balance {
			(slash_balance, reporter_deposit.saturating_sub(slash_balance))
		} else {
			(*reporter_deposit, Zero::zero())
		};
		*reporter_deposit = new_deposit;
		*storage_pot_reserved = storage_pot_reserved.saturating_add(slash_reserved);
	}

	fn round_reported(round: RoundIndex, node: &T::AccountId) -> bool {
		if round.is_zero() {
			return true;
		} 
		if RoundsReport::<T>::contains_key(round, node) {
			return true;
		}
		Nodes::<T>::get(&node).map(|v| v.last_round.is_zero()).unwrap_or_default()
	}

	fn clear_store_file(cid: &FileId) {
		StoreFiles::<T>::remove(cid);
		FileOrders::<T>::remove(cid);
		Self::deposit_event(Event::<T>::StoreFileRemoved(cid.clone()));
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

    fn get_next_round_bn() -> BlockNumberFor<T> {
        let current_round = CurrentRound::<T>::get();
        RoundsBlockNumber::<T>::get(current_round) + T::RoundDuration::get()
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

    let vk: Vec<u8> = [
        &vec![4][..],
        &pk[..]
    ].concat();

	if let (Ok(sig), Ok(vk)) = (Signature::from_bytes(&sig), VerifyingKey::from_sec1_bytes(&vk[..])) {
		return vk.verify(data, &sig).is_ok()
	}
	false
}

fn encode_u64(number: u64) -> Vec<u8> {
    let mut value = number;
    let mut encoded_number: Vec<u8> = [].to_vec();
    loop {
        encoded_number.push((value%10) as u8 + 48u8); // "0" is 48u8
        value /= 10;
        if value == 0 {
            break;
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