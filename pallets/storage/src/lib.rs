//! # Storage Online Module

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(any(test, feature = "runtime-benchmarks"))]
mod sign;

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

mod constants;

pub use constants::*;

pub mod weights;

pub mod migrations;

use codec::{Decode, Encode};
use frame_support::{
	traits::{Currency, ExistenceRequirement, Get, OnUnbalanced, ReservableCurrency, UnixTime},
	weights::Weight,
	PalletId,
};
use frame_system::{pallet_prelude::BlockNumberFor, Config as SystemConfig};
use p256::ecdsa::{
	signature::{Signature, Verifier},
	VerifyingKey,
};
use runtime_api::NodeDepositInfo;
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
pub type SessionIndex = u32;
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;
pub type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;
pub type NodeInfoOf<T> = NodeInfo<<T as SystemConfig>::AccountId, BalanceOf<T>, BlockNumberFor<T>>;
pub type FileInfoOf<T> = FileInfo<<T as SystemConfig>::AccountId, BalanceOf<T>, BlockNumberFor<T>>;
pub type SessionStateOf<T> = SessionState<BlockNumberFor<T>>;

pub use pallet::*;
pub use weights::WeightInfo;

/// Tee node info
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct NodeInfo<AccountId, Balance, BlockNumber> {
	/// Stash account
	pub stash: AccountId,
	/// Stash deposit
	pub deposit: Balance,
	/// Node's machine id
	pub machine_id: Option<MachineId>,
	/// A increment id of one report
	pub rid: u64,
	/// Effective storage space
	pub used: u64,
	/// Mine power of node, use this to distribute mining rewards
	pub power: u64,
	/// Slash Effective storage space
	pub slash_used: u64,
	/// Reward from liquidation
	pub reward: Balance,
	/// Latest report at
	pub reported_at: BlockNumber,
	/// Prev session reported at
	pub prev_reported_at: BlockNumber,
}

impl<AccountId: sp_std::fmt::Debug, Balance: Zero, BlockNumber: Zero>
	NodeInfo<AccountId, Balance, BlockNumber>
{
	pub fn new(account_id: AccountId) -> Self {
		log::warn!(target: "runtime::file-storage", "Invalid replica on node {:?}", account_id);
		Self {
			stash: account_id,
			deposit: Zero::zero(),
			machine_id: None,
			rid: 0,
			used: 0,
			power: 0,
			slash_used: 0,
			reward: Zero::zero(),
			reported_at: Zero::zero(),
			prev_reported_at: Zero::zero(),
		}
	}
}

/// Session state
#[derive(Clone, Encode, Decode, Eq, PartialEq, Default, RuntimeDebug, TypeInfo)]
pub struct SessionState<BlockNumber> {
	/// Current session number
	pub current: SessionIndex,
	/// Prev session begin at
	pub prev_begin_at: BlockNumber,
	/// Session begin at
	pub begin_at: BlockNumber,
	/// Session end at
	pub end_at: BlockNumber,
}

/// Session summary info
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct SummaryInfo<Balance> {
	/// Number of reproted nodes
	pub count: u32,
	/// Node's storage power
	#[codec(compact)]
	pub power: u128,
	/// Eeffictive storage size
	#[codec(compact)]
	pub used: u128,
	/// Reward for node power
	pub mine_reward: Balance,
	/// Reward for node store file
	pub store_reward: Balance,
	/// How many mine reward that already assigned to the node
	pub paid_mine_reward: Balance,
	/// How many store reward that already assigned to the node
	pub paid_store_reward: Balance,
}

/// Ipfs file
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct FileInfo<AccountId, Balance, BlockNumber> {
	/// Funds gathered in this file
	pub reserved: Balance,
	/// Basic cost of sumit to network
	pub base_fee: Balance,
	// Store file size
	pub file_size: u64,
	// When store file
	pub add_at: BlockNumber,
	/// The cost of storing for a period of time
	pub fee: Balance,
	/// When the order need to close or renew
	pub liquidate_at: BlockNumber,
	/// Nodes store the file
	pub replicas: Vec<AccountId>,
}

/// Node register info
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct RegisterInfo {
	/// PUb key to verify signed message
	pub key: PubKey,
	/// Tee enclave id
	pub enclave: EnclaveId,
}

// A value placed in storage that represents the current version of the Scheduler storage.
// This value is used by the `on_runtime_upgrade` logic to determine whether we run
// storage migration logic.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum Releases {
	V0,
	V1,
}

impl Default for Releases {
	fn default() -> Self {
		Releases::V0
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The module id, used for deriving its sovereign account ID.
		type PalletId: Get<PalletId>;

		/// The origin which may forcibly remove file. Root can always do this.
		type ForceOrigin: EnsureOrigin<Self::Origin>;

		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// Time used for validating register cert
		type UnixTime: UnixTime;

		/// The basic amount of funds that slashed when node is offline or misbehavier
		#[pallet::constant]
		type SlashBalance: Get<BalanceOf<Self>>;

		/// Number of blocks that node's need report its work
		#[pallet::constant]
		type SessionDuration: Get<BlockNumberFor<Self>>;

		/// Number of blocks to liquidate a file
		#[pallet::constant]
		type LiquidateDuration: Get<BlockNumberFor<Self>>;

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
		type FileSizePrice: Get<BalanceOf<Self>>;

		/// The ratio for divide direct store reward and share store reward
		#[pallet::constant]
		type StoreRewardRatio: Get<Perbill>;

		/// Number fo founds to stash for registering a node
		#[pallet::constant]
		type StashBalance: Get<BalanceOf<Self>>;

		/// Mine factor
		#[pallet::constant]
		type MineFactor: Get<Perbill>;

		/// The maximum amount of  mine reward in each session
		#[pallet::constant]
		type MaxMineReward: Get<BalanceOf<Self>>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	/// The Tee enclaves
	#[pallet::storage]
	pub type Enclaves<T: Config> = StorageMap<_, Twox64Concat, EnclaveId, BlockNumberFor<T>>;

	/// Number of balance reserved to storage pot
	#[pallet::storage]
	pub type StoragePotReserved<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// Node information
	#[pallet::storage]
	pub type Nodes<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, NodeInfoOf<T>>;

	/// Node register information
	#[pallet::storage]
	pub type Registers<T: Config> = StorageMap<_, Twox64Concat, MachineId, RegisterInfo>;

	/// Current session state
	#[pallet::storage]
	pub type Session<T: Config> = StorageValue<_, SessionStateOf<T>, ValueQuery>;

	/// Summary info of earch session
	#[pallet::storage]
	pub type Summarys<T: Config> =
		StorageMap<_, Twox64Concat, SessionIndex, SummaryInfo<BalanceOf<T>>, ValueQuery>;

	/// Information for file orders
	#[pallet::storage]
	pub type Files<T: Config> = StorageMap<_, Blake2_128Concat, FileId, FileInfoOf<T>>;

	/// Storage version of the pallet.
	///
	/// New networks start with last version.
	#[pallet::storage]
	pub type StorageVersion<T: Config> = StorageValue<_, Releases, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Add or change enclave.
		SetEnclave { enclave_id: EnclaveId, expire_at: BlockNumberFor<T> },
		/// A account have been stashed.
		Stashed { controller: T::AccountId, amount: BalanceOf<T> },
		/// A account have withdrawn some founds.
		Withdrawn { controller: T::AccountId, stash: T::AccountId, amount: BalanceOf<T> },
		/// A node was registerd.
		NodeRegistered { controller: T::AccountId, machine_id: MachineId },
		/// A node reported its work.
		NodeReported {
			controller: T::AccountId,
			machine_id: MachineId,
			mine_reward: BalanceOf<T>,
			share_store_reward: BalanceOf<T>,
			direct_store_reward: BalanceOf<T>,
			slash: BalanceOf<T>,
		},
		/// A request to store file.
		FileAdded { cid: FileId, caller: T::AccountId, fee: BalanceOf<T>, first: bool },
		/// A file have been removed.
		FileDeleted { cid: FileId },
		/// A node have stored file
		FileStored { cid: FileId },
		/// A file was deleted by admin.
		FileForceDeleted { cid: FileId },
		/// A session end
		NewSession { index: SessionIndex, mine: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Enclave's expire time should not great than current
		EnclaveExpired,
		/// Node have been stashed with another account
		NotPair,
		/// Node's deposit is not enough to withdraw
		NoEnoughToWithdraw,
		/// Node Have not stashed
		NodeNotStashed,
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
		/// Already reported in current session
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
		InsufficientDeposit,
		/// Invalid cid
		InvalidCid,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> frame_support::weights::Weight {
			let session = Session::<T>::get();
			if now > session.end_at {
				Self::session_end()
			} else {
				0
			}
		}

		fn on_runtime_upgrade() -> Weight {
			if StorageVersion::<T>::get() == Releases::V0 {
				migrations::v1::migrate::<T>()
			} else {
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<(), &'static str> {
			if StorageVersion::<T>::get() == Releases::V0 {
				migrations::v1::pre_migrate::<T>()
			} else {
				Ok(())
			}
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade() -> Result<(), &'static str> {
			migrations::v1::post_migrate::<T>()
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
			StorageVersion::<T>::put(Releases::V1);
			Session::<T>::mutate(|session| {
				session.begin_at = One::one();
				session.end_at = T::SessionDuration::get();
			});
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
				ensure!(expire_at < old_expire_at, Error::<T>::EnclaveExpired);
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
			controller: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			let controller = T::Lookup::lookup(controller)?;
			let stash_balance = T::StashBalance::get();
			if let Some(mut node_info) = Nodes::<T>::get(&controller) {
				ensure!(&node_info.stash == &stash, Error::<T>::NotPair);
				let new_deposit =
					Self::deposit_for_used(node_info.used).saturating_add(stash_balance);
				let amount = new_deposit.saturating_sub(node_info.deposit);
				if !amount.is_zero() {
					T::Currency::transfer(
						&stash,
						&Self::account_id(),
						amount,
						ExistenceRequirement::KeepAlive,
					)?;
					node_info.deposit = new_deposit;
					Nodes::<T>::insert(controller.clone(), node_info);
					Self::deposit_event(Event::<T>::Stashed { controller, amount });
				}
			} else {
				T::Currency::transfer(
					&stash,
					&Self::account_id(),
					stash_balance,
					ExistenceRequirement::KeepAlive,
				)?;
				Nodes::<T>::insert(
					&controller,
					NodeInfo {
						stash,
						deposit: stash_balance,
						machine_id: None,
						rid: 0,
						used: 0,
						power: 0,
						slash_used: 0,
						reward: Zero::zero(),
						reported_at: Zero::zero(),
						prev_reported_at: Zero::zero(),
					},
				);
				Self::deposit_event(Event::<T>::Stashed { controller, amount: stash_balance });
			}
			Ok(())
		}

		/// Withdraw the mine reward, node's despoist should not below T::StashBalance
		#[pallet::weight(T::WeightInfo::withdraw())]
		pub fn withdraw(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let mut node_info = Nodes::<T>::get(&controller).ok_or(Error::<T>::NodeNotStashed)?;
			let stash_balance = T::StashBalance::get();
			let new_deposit = Self::deposit_for_used(node_info.used).saturating_add(stash_balance);
			let amount = node_info.deposit.saturating_sub(new_deposit);
			ensure!(!amount.is_zero(), Error::<T>::NoEnoughToWithdraw);
			node_info.deposit = new_deposit;
			let stash = node_info.stash.clone();
			T::Currency::transfer(
				&Self::account_id(),
				&stash,
				amount,
				ExistenceRequirement::KeepAlive,
			)?;
			Nodes::<T>::insert(controller.clone(), node_info);
			Self::deposit_event(Event::<T>::Withdrawn { controller, stash, amount });
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
			let controller = ensure_signed(origin)?;
			let maybe_register_info = Registers::<T>::get(&machine_id);
			let mut node_info = Nodes::<T>::get(&controller).ok_or(Error::<T>::NodeNotStashed)?;
			if maybe_register_info.is_some() {
				ensure!(&node_info.machine_id.is_some(), Error::<T>::MachineAlreadyRegistered);
			}
			if let Some(stash_machine_id) = &node_info.machine_id {
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
			let now_at = Self::now_at();
			let enclave = &isv_quote_body[112..144].to_vec();
			ensure!(
				Enclaves::<T>::get(enclave).unwrap_or_default() > now_at,
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
					node_info.machine_id = Some(machine_id.clone());
					Nodes::<T>::insert(&controller, node_info);
				},
			}

			Self::deposit_event(Event::<T>::NodeRegistered { controller, machine_id });
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
			liquidate_fils: Vec<FileId>,
		) -> DispatchResult {
			let reporter = ensure_signed(origin)?;
			ensure!(
				add_files.len() <= T::MaxReportFiles::get() as usize ||
					liquidate_fils.len() <= T::MaxReportFiles::get() as usize,
				Error::<T>::ReportExceedLimit
			);
			let mut node_info = Nodes::<T>::get(&reporter).ok_or(Error::<T>::NodeNotStashed)?;
			let machine_id =
				node_info.machine_id.as_ref().ok_or(Error::<T>::UnregisterNode)?.clone();

			ensure!(node_info.deposit >= T::SlashBalance::get(), Error::<T>::InsufficientDeposit);

			let register = Registers::<T>::get(&machine_id).ok_or(Error::<T>::UnregisterNode)?;
			let now_at = Self::now_at();
			let enclave_bn =
				Enclaves::<T>::get(&register.enclave).ok_or(Error::<T>::InvalidEnclave)?;
			ensure!(now_at <= enclave_bn, Error::<T>::InvalidEnclave);
			let session = Session::<T>::get();
			let current = session.current;
			let prev = current.saturating_sub(One::one());
			ensure!(!Self::is_reported(&node_info, &session), Error::<T>::DuplicateReport);
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

			let mut ctx: ReportContextOf<T> = ReportContext {
				now_at,
				session: session.clone(),
				reporter: reporter.clone(),
				storage_pot_add: Zero::zero(),
				node_changes: BTreeMap::new(),
				node_infos: BTreeMap::new(),
				session_store_reward: Zero::zero(),
			};
			let mut slash: BalanceOf<T> = Zero::zero();

			for cid in liquidate_fils.iter() {
				Self::report_liquidate_file(&mut ctx, cid);
			}

			for (cid, file_size, ..) in add_files.iter() {
				if file_size > &T::MaxFileSize::get() {
					continue
				}
				Self::report_add_file(&mut ctx, cid, *file_size);
			}
			for cid in del_files.iter() {
				Self::report_delete_file(&mut ctx, cid);
			}

			let mut mine_reward: BalanceOf<T> = Zero::zero();
			let mut share_store_reward: BalanceOf<T> = Zero::zero();
			if Self::is_prev_reported(&node_info, &session) {
				Summarys::<T>::mutate(prev, |mut summary| {
					let used_ratio = Perbill::from_rational(node_info.used as u128, summary.used);
					let power_ratio =
						Perbill::from_rational(node_info.power as u128, summary.power);
					share_store_reward = used_ratio * summary.store_reward;
					mine_reward = power_ratio * summary.mine_reward;
					summary.paid_store_reward =
						summary.paid_store_reward.saturating_add(share_store_reward);
					summary.paid_mine_reward = summary.paid_mine_reward.saturating_add(mine_reward);
				});
			} else {
				if !node_info.reported_at.is_zero() {
					let slash_balance = T::SlashBalance::get();
					if !slash_balance.is_zero() {
						slash = slash.saturating_add(slash_balance);
					}
					ctx.storage_pot_add = ctx.storage_pot_add.saturating_add(node_info.reward);
					node_info.reward = Zero::zero();
				}
			}

			let reporter_change = ctx.node_changes.entry(reporter.clone()).or_default();
			node_info.used = node_info
				.used
				.saturating_add(reporter_change.used_inc)
				.saturating_sub(reporter_change.slash_used_dec)
				.saturating_sub(reporter_change.used_dec);
			let reporter_total_used_dec =
				node_info.slash_used.saturating_add(reporter_change.slash_used_dec);
			if !reporter_total_used_dec.is_zero() {
				slash = slash.saturating_add(Self::deposit_for_used(reporter_total_used_dec));
			}
			let direct_store_reward = node_info.reward.saturating_add(reporter_change.reward);

			let reporter_new_deposit = node_info
				.deposit
				.saturating_add(mine_reward)
				.saturating_add(share_store_reward)
				.saturating_add(direct_store_reward);
			let (reporter_new_deposit, storage_pot_add_slash) = if reporter_new_deposit >= slash {
				(reporter_new_deposit.saturating_sub(slash), slash)
			} else {
				(Zero::zero(), reporter_new_deposit)
			};
			ctx.storage_pot_add = ctx.storage_pot_add.saturating_add(storage_pot_add_slash);
			node_info.deposit = reporter_new_deposit;
			node_info.slash_used = 0;
			node_info.reward = Zero::zero();

			let mut storage_pot_add_rewards: BalanceOf<T> = Zero::zero();
			for (account, node_change) in ctx.node_changes.iter() {
				if account != &reporter {
					let ReportNodeChange { slash_used_dec, used_dec, used_inc, reward } =
						node_change;
					Nodes::<T>::mutate(account, |maybe_node| {
						if let Some(other_node) = maybe_node {
							other_node.slash_used =
								other_node.slash_used.saturating_add(*slash_used_dec);
							other_node.used = other_node
								.used
								.saturating_add(*used_inc)
								.saturating_sub(*slash_used_dec)
								.saturating_sub(*used_dec);
							other_node.reward = other_node.reward.saturating_add(*reward);
						} else {
							storage_pot_add_rewards =
								storage_pot_add_rewards.saturating_add(*reward);
						}
					})
				}
			}

			ctx.storage_pot_add = ctx.storage_pot_add.saturating_add(storage_pot_add_rewards);
			node_info.power = power.min(T::MaxPower::get());

			node_info.rid = rid;
			node_info.prev_reported_at = node_info.reported_at;
			node_info.reported_at = now_at;

			StoragePotReserved::<T>::mutate(|v| *v = v.saturating_add(ctx.storage_pot_add));

			Summarys::<T>::mutate(current, |summary| {
				summary.count += 1;
				summary.used = summary.used.saturating_add(node_info.used.saturated_into());
				summary.power = summary.power.saturating_add(node_info.power.saturated_into());
				summary.store_reward =
					summary.store_reward.saturating_add(ctx.session_store_reward);
			});
			Nodes::<T>::insert(reporter.clone(), node_info);

			Self::deposit_event(Event::<T>::NodeReported {
				controller: reporter,
				machine_id,
				mine_reward,
				share_store_reward,
				direct_store_reward,
				slash,
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
			ensure!(is_cid(&cid), Error::<T>::InvalidCid);

			if let Some(mut file) = Files::<T>::get(&cid) {
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
				Files::<T>::insert(cid.clone(), file);
				Self::deposit_event(Event::<T>::FileAdded { cid, caller: who, fee, first: false });
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
				Files::<T>::insert(
					cid.clone(),
					FileInfo {
						reserved: fee.saturating_sub(base_fee),
						base_fee,
						file_size,
						add_at: Self::now_at(),
						fee: Zero::zero(),
						liquidate_at: Zero::zero(),
						replicas: vec![],
					},
				);
				Self::deposit_event(Event::<T>::FileAdded { cid, caller: who, fee, first: true });
			}
			Ok(())
		}

		/// Force delete unsoloved file
		#[pallet::weight(T::WeightInfo::force_delete())]
		pub fn force_delete(origin: OriginFor<T>, cid: FileId) -> DispatchResult {
			T::ForceOrigin::ensure_origin(origin)?;

			if let Some(file) = Files::<T>::get(&cid) {
				let now = Self::now_at();
				ensure!(
					!file.base_fee.is_zero() &&
						now > file.add_at.saturating_add(T::LiquidateDuration::get()),
					Error::<T>::UnableToDeleteFile
				);
				StoragePotReserved::<T>::mutate(|v| {
					*v = v.saturating_add(file.base_fee).saturating_add(file.reserved)
				});
				Files::<T>::remove(&cid);
				Self::deposit_event(Event::<T>::FileForceDeleted { cid });
			}
			Ok(())
		}
	}
}

type ReportContextOf<T> = ReportContext<
	<T as SystemConfig>::AccountId,
	<<T as Config>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance,
	<T as SystemConfig>::BlockNumber,
>;

#[derive(RuntimeDebug)]
struct ReportContext<AccountId, Balance, BlockNumber> {
	now_at: BlockNumber,
	session: SessionState<BlockNumber>,
	reporter: AccountId,
	storage_pot_add: Balance,
	node_changes: BTreeMap<AccountId, ReportNodeChange<Balance>>,
	node_infos: BTreeMap<AccountId, NodeInfo<AccountId, Balance, BlockNumber>>,
	session_store_reward: Balance,
}

#[derive(RuntimeDebug, Default)]
struct ReportNodeChange<Balance> {
	slash_used_dec: u64,
	used_dec: u64,
	used_inc: u64,
	reward: Balance,
}

impl<T: Config> Pallet<T> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	pub fn store_fee(file_size: u64, time: BlockNumberFor<T>) -> BalanceOf<T> {
		let duration = T::LiquidateDuration::get();
		let mut sessions: u64 = (time / duration).saturated_into();
		let rem = time % duration;
		if !rem.is_zero() {
			sessions += 1;
		}
		Self::store_file_bytes_fee(file_size)
			.saturating_mul(sessions.saturated_into())
			.saturating_add(T::FileBaseFee::get())
	}

	pub fn node_deposit(controller: &T::AccountId) -> NodeDepositInfo<BalanceOf<T>> {
		let stash_balance = T::StashBalance::get();
		if let Some(node_info) = Nodes::<T>::get(&controller) {
			let used_deposit = Self::deposit_for_used(node_info.used);
			NodeDepositInfo {
				current_deposit: node_info.deposit,
				slash_deposit: stash_balance,
				used_deposit,
			}
		} else {
			NodeDepositInfo { slash_deposit: stash_balance, ..Default::default() }
		}
	}

	pub(crate) fn session_end() -> Weight {
		let session = Session::<T>::get();
		let current = session.current;
		let (mine_reward, storage_pot_reserved, require_mine) = Self::calculate_mine(current);
		if !mine_reward.is_zero() {
			Summarys::<T>::mutate(current, |session| {
				session.mine_reward = session.mine_reward.saturating_add(mine_reward);
			});
		}
		StoragePotReserved::<T>::mutate(|v| *v = storage_pot_reserved);
		if !require_mine.is_zero() {
			T::Currency::deposit_creating(&Self::account_id(), require_mine);
		}

		if current >= 2 {
			Summarys::<T>::remove(current.saturating_sub(2));
		}
		Session::<T>::mutate(|session| {
			let duration = T::SessionDuration::get();
			session.prev_begin_at = session.begin_at;
			session.begin_at = session.begin_at.saturating_add(duration);
			session.end_at = session.end_at.saturating_add(duration);
			session.current = session.current + 1;
		});
		Self::deposit_event(Event::<T>::NewSession { index: current + 1, mine: require_mine });
		T::WeightInfo::session_end()
	}

	pub(crate) fn calculate_mine(
		index: SessionIndex,
	) -> (BalanceOf<T>, BalanceOf<T>, BalanceOf<T>) {
		let summary = Summarys::<T>::get(index);
		let mine_reward = if summary.power.is_zero() {
			Zero::zero()
		} else {
			T::MaxMineReward::get().min((T::MineFactor::get() * summary.power).saturated_into())
		};
		let prev_summary = Summarys::<T>::get(index.saturating_sub(1));
		let unpaid = prev_summary
			.store_reward
			.saturating_add(prev_summary.mine_reward)
			.saturating_sub(prev_summary.paid_mine_reward)
			.saturating_sub(prev_summary.paid_store_reward);
		let withdraw = unpaid.saturating_add(StoragePotReserved::<T>::get());
		let (new_reserved, require_mine) = if withdraw >= mine_reward {
			(withdraw.saturating_sub(mine_reward), Zero::zero())
		} else {
			(Zero::zero(), mine_reward.saturating_sub(withdraw))
		};
		(mine_reward, new_reserved, require_mine)
	}

	fn report_add_file(ctx: &mut ReportContextOf<T>, cid: &FileId, file_size: u64) {
		if let Some(mut file) = Files::<T>::get(cid) {
			if !file.liquidate_at.is_zero() {
				let mut new_nodes = vec![];
				let mut is_included = false;
				for (index, replica_account) in file.replicas.iter().enumerate() {
					let replica_node =
						ctx.node_infos.entry(replica_account.clone()).or_insert_with(|| {
							Nodes::<T>::get(replica_account)
								.unwrap_or_else(|| NodeInfo::new(replica_account.clone()))
						});
					if Self::is_prev_reported(replica_node, &ctx.session) {
						new_nodes.push(replica_account.clone());
					} else {
						let node_change =
							ctx.node_changes.entry(replica_account.clone()).or_default();
						if (index as u32) < T::MaxFileReplicas::get() {
							node_change.slash_used_dec =
								node_change.slash_used_dec.saturating_add(file_size);
						} else {
							node_change.used_dec = node_change.used_dec.saturating_add(file_size);
						}
					}
					if replica_account == &ctx.reporter {
						is_included = true;
					}
				}
				if !is_included && (new_nodes.len() as u32) < T::MaxFileReplicas::get() {
					new_nodes.push(ctx.reporter.clone());
					let node_change = ctx.node_changes.entry(ctx.reporter.clone()).or_default();
					node_change.used_inc = node_change.used_inc.saturating_add(file_size);
				}
				file.replicas = new_nodes;
				Files::<T>::insert(cid, file);
			} else {
				let is_file_exist = Self::liquidate_file(
					ctx,
					cid,
					&mut file,
					vec![ctx.reporter.clone()],
					Some(file_size),
				);
				if is_file_exist {
					let node_change = ctx.node_changes.entry(ctx.reporter.clone()).or_default();
					node_change.used_inc = node_change.used_inc.saturating_add(file_size);
				}
			}
		}
	}

	fn report_delete_file(ctx: &mut ReportContextOf<T>, cid: &FileId) {
		if let Some(mut file) = Files::<T>::get(cid) {
			if let Ok(index) = file.replicas.binary_search(&ctx.reporter) {
				file.replicas.remove(index);
				let node_change = ctx.node_changes.entry(ctx.reporter.clone()).or_default();
				if (index as u32) < T::MaxFileReplicas::get() {
					node_change.slash_used_dec =
						node_change.slash_used_dec.saturating_add(file.file_size);
				} else {
					node_change.used_dec = node_change.used_dec.saturating_add(file.file_size);
				}
				Files::<T>::insert(cid, file);
			}
		}
	}

	fn report_liquidate_file(ctx: &mut ReportContextOf<T>, cid: &FileId) {
		if let Some(mut file) = Files::<T>::get(cid) {
			if file.liquidate_at.is_zero() || file.liquidate_at > ctx.now_at {
				return
			}

			let file_fee = file.fee;
			let mut total_order_reward: BalanceOf<T> = Zero::zero();
			let each_order_reward = Self::share_ratio() * file_fee;
			let mut replicas = vec![];
			for (index, replica_account) in file.replicas.iter().enumerate() {
				let replica_node =
					ctx.node_infos.entry(replica_account.clone()).or_insert_with(|| {
						Nodes::<T>::get(replica_account)
							.unwrap_or_else(|| NodeInfo::new(replica_account.clone()))
					});
				if Self::is_prev_reported(replica_node, &ctx.session) {
					let node_change = ctx.node_changes.entry(replica_account.clone()).or_default();
					node_change.reward = node_change.reward.saturating_add(each_order_reward);
					total_order_reward = total_order_reward.saturating_add(each_order_reward);
					if replica_account == &ctx.reporter {
						node_change.reward = node_change.reward.saturating_add(each_order_reward);
						total_order_reward = total_order_reward.saturating_add(each_order_reward);
					}
					replicas.push(replica_account.clone());
				} else {
					let node_change = ctx.node_changes.entry(replica_account.clone()).or_default();
					if (index as u32) < T::MaxFileReplicas::get() {
						node_change.slash_used_dec =
							node_change.slash_used_dec.saturating_add(file.file_size);
					} else {
						node_change.used_dec = node_change.used_dec.saturating_add(file.file_size);
					}
				}
			}
			let is_file_exist = Self::liquidate_file(ctx, cid, &mut file, replicas.clone(), None);
			if !is_file_exist {
				for node in replicas.iter() {
					let node_change = ctx.node_changes.entry(node.clone()).or_default();
					node_change.used_dec = node_change.used_dec.saturating_add(file.file_size);
				}
			}
			let unpaid_reward = file_fee.saturating_sub(total_order_reward);
			if !unpaid_reward.is_zero() {
				ctx.session_store_reward = ctx.session_store_reward.saturating_add(unpaid_reward);
			}
		}
	}

	fn liquidate_file(
		ctx: &mut ReportContextOf<T>,
		cid: &FileId,
		file: &mut FileInfoOf<T>,
		nodes: Vec<T::AccountId>,
		maybe_file_size: Option<u64>,
	) -> bool {
		let first = file.liquidate_at.is_zero();
		let expect_order_fee =
			Self::store_file_bytes_fee(maybe_file_size.unwrap_or(file.file_size));
		if let Some(file_size) = maybe_file_size {
			if first {
				ctx.storage_pot_add = ctx.storage_pot_add.saturating_add(file.base_fee);
				// user underreported the file size
				if file.file_size < file_size && file.reserved < expect_order_fee {
					let to_reporter_reward = Self::share_ratio() * file.reserved;
					let node_change = ctx.node_changes.entry(ctx.reporter.clone()).or_default();
					node_change.reward = node_change.reward.saturating_add(to_reporter_reward);
					ctx.session_store_reward = ctx
						.session_store_reward
						.saturating_add(file.reserved.saturating_sub(to_reporter_reward));
					Self::delete_file(cid);
					return false
				}
				file.base_fee = Zero::zero();
				file.file_size = file_size;
			}
		}
		let (order_fee, new_reserved) = if file.reserved > expect_order_fee {
			(expect_order_fee, file.reserved.saturating_sub(expect_order_fee))
		} else {
			(file.reserved, Zero::zero())
		};
		if order_fee.is_zero() {
			Self::delete_file(cid);
			return false
		}
		let now_at = Self::now_at();
		let mut duration = T::LiquidateDuration::get();
		if order_fee < expect_order_fee {
			duration = Perbill::from_rational(order_fee, expect_order_fee) * duration;
		}
		if first {
			Self::deposit_event(Event::<T>::FileStored { cid: cid.clone() });
		}

		file.fee = order_fee;
		file.file_size = file.file_size;
		file.liquidate_at = now_at.saturating_add(duration);
		file.replicas = nodes;
		file.reserved = new_reserved;
		Files::<T>::insert(cid, file);
		return true
	}

	fn is_reported(node_info: &NodeInfoOf<T>, session: &SessionStateOf<T>) -> bool {
		!node_info.reported_at.is_zero() && node_info.reported_at >= session.begin_at
	}

	fn is_prev_reported(node_info: &NodeInfoOf<T>, session: &SessionStateOf<T>) -> bool {
		if session.prev_begin_at.is_zero() {
			return true
		}
		if Self::is_reported(node_info, session) {
			return node_info.prev_reported_at >= session.prev_begin_at
		} else {
			return node_info.reported_at >= session.prev_begin_at
		}
	}

	fn delete_file(cid: &FileId) {
		Files::<T>::remove(cid);
		Self::deposit_event(Event::<T>::FileDeleted { cid: cid.clone() });
	}

	/// Reserved deposit balance for node's used storage space
	fn deposit_for_used(space: u64) -> BalanceOf<T> {
		Self::share_ratio() * Self::store_file_bytes_fee(space)
	}

	fn store_file_fee(file_size: u64) -> BalanceOf<T> {
		T::FileBaseFee::get().saturating_add(Self::store_file_bytes_fee(file_size))
	}

	fn share_ratio() -> Perbill {
		Perbill::from_rational(1, T::EffectiveFileReplicas::get().saturating_add(1)) *
			T::StoreRewardRatio::get()
	}

	fn store_file_bytes_fee(file_size: u64) -> BalanceOf<T> {
		let mut file_size_in_mega = file_size / 1_048_576;
		if file_size % 1_048_576 != 0 {
			file_size_in_mega += 1;
		}
		T::FileSizePrice::get().saturating_mul(file_size_in_mega.saturated_into())
	}

	fn now_at() -> BlockNumberFor<T> {
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

fn is_cid(cid: &[u8]) -> bool {
	let len = cid.len();
	if len == 46 {
		return &cid[0..2] == b"Qm" &&
			cid[2..].iter().all(|x| match *x {
				49..=57 | 65..=90 | 97..=122 => true,
				_ => false,
			})
	} else if len == 59 {
		return &cid[0..7] == b"bafkrei" &&
			cid[7..].iter().all(|x| match *x {
				49..=57 | 65..=90 | 97..=122 => true,
				_ => false,
			})
	}
	false
}
