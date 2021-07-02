//! # Storage Online Module

#![cfg_attr(not(feature = "std"), no_std)]


// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
// #[cfg(test)]
// pub mod mock;
// #[cfg(test)]
// mod tests;

// pub mod weights;


use sp_std::{prelude::*, collections::{btree_set::BTreeSet, btree_map::BTreeMap}};
use sp_runtime::{RuntimeDebug, ArithmeticError, traits::{Zero, StaticLookup, Saturating, CheckedAdd, CheckedSub}};
use codec::{Encode, Decode, HasCompact};
use frame_support::{
	ensure,
	traits::{Currency, ReservableCurrency},
	dispatch::DispatchResult,
};
use frame_system::{Config as SystemConfig, pallet_prelude::BlockNumberFor};

pub type RootId = Vec<u8>;
pub type EnclaveId = Vec<u8>;
pub type PubKey = Vec<u8>;
pub type ReportRound = u64;
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;

// pub use weights::WeightInfo;
pub use pallet::*;

pub const LOG_TARGET: &'static str = "runtime::storage";

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

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct ReporterInfo {
    pub round: u64,
    pub seal_root: RootId,
    pub files_root: RootId,
	pub stats: StorageStats,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct RegisterInfo {
	pub enclave: EnclaveId,
	pub key: Option<PubKey>,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct NodeInfo {
	pub key: PubKey,
	pub slash_defer_rounds: ReportRound,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default)]
pub struct StorageStats {
	pub used: u128,
	pub free: u128,
	pub files_size: u128,
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{BoundedVec, pallet_prelude::*};
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

		type Currency: ReservableCurrency<Self::AccountId>;

		#[pallet::constant]
		type SlashDeferDuration: Get<ReportRound>;

		#[pallet::constant]
		type BlocksPerRound: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type FileDuration: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type FileReplica: Get<u32>;

		#[pallet::constant]
		type FilePrice: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxTrashSize: Get<u128>;

		#[pallet::constant]
		type MaxFileSize: Get<u64>;
	}

	#[pallet::type_value]
	pub fn RoundOnEmpty() -> ReportRound { 0 }

	#[pallet::type_value]
	pub fn StorageStatsOnEmpty() -> StorageStats {
		Default::default()
	}

	#[pallet::storage]
	#[pallet::getter(fn enclave)]
	pub type Enclave<T: Config> = StorageMap<
		_,
		Twox64Concat,
		EnclaveId,
		BlockNumberFor<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn node)]
	pub type Node<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		NodeInfo,
	>;

	#[pallet::storage]
	#[pallet::getter(fn register)]
	pub type Register<T: Config> = StorageMap<
		_,
		Twox64Concat,
		PubKey,
		RegisterInfo,
	>;

	#[pallet::storage]
	#[pallet::getter(fn reporter)]
	pub type Reporter<T: Config> = StorageMap<
		_,
		Twox64Concat,
		PubKey,
		ReporterInfo,
	>;

	#[pallet::storage]
	#[pallet::getter(fn round)]
	pub type Round<T: Config> = StorageValue<_, ReportRound, ValueQuery, RoundOnEmpty>;

	#[pallet::storage]
	#[pallet::getter(fn stats)]
	pub type Stats<T: Config> = StorageValue<_, StorageStats, ValueQuery, StorageStatsOnEmpty>;

	#[pallet::storage]
	#[pallet::getter(fn reported_in_round)]
	pub type ReportedInRound<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat, PubKey,
		Twox64Concat, ReportRound,
		bool,
	>;


	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
	pub enum Event<T: Config> {
	}

	#[pallet::error]
	pub enum Error<T> {

	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> frame_support::weights::Weight {
			if (now % T::BlocksPerRound::get()).is_zero() {
				Self::update_nodes();
			}
			// TODO: weights
			0
		}
	}


	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight((1_000_000, DispatchClass::Operational))]
		pub fn set_enclave(
			origin: OriginFor<T>,
			enclave: EnclaveId,
			expire: T::BlockNumber,
		) -> DispatchResult {
			todo!()
		}

		#[pallet::weight(1_000_000)]
		pub fn register_node(
			origin: OriginFor<T>,
			cert: Vec<u8>,
			body: Vec<u8>,
			sig: Vec<u8>,
			account: Vec<u8>,
			checksum: Vec<u8>,
		) -> DispatchResult {
			todo!()
		}

		#[pallet::weight(1_000_000)]
		pub fn report_works(
			origin: OriginFor<T>,
			pk1: PubKey,
			pk2: PubKey,
			bn: u64,
			bh: Vec<u8>,
			seal_size: u64,
			fiels_size: u64,
			added_files: Vec<(RootId, u64, u64)>,
			deleted_files: Vec<(RootId, u64, u64)>,
			seal_root: RootId,
			files_root: RootId,
			sig: Vec<u8>,
		) -> DispatchResult {
			todo!()
		}

		#[pallet::weight(1_000_000)]
		pub fn set_stash(
			origin: OriginFor<T>,
			controller: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			todo!()
		}

		#[pallet::weight(0)]
		pub fn report_offline(
			origin: OriginFor<T>,
			stash: T::AccountId,
			at: T::BlockNumber
		) -> DispatchResult {
			todo!()
		}

		#[pallet::weight(1_000_000)]
		pub fn add_order(
			origin: OriginFor<T>,
			cid: RootId,
			file_size: u64,
			duration: T::BlockNumber,
			reserve: BalanceOf<T>,
		) -> DispatchResult {
			todo!()
		}

		#[pallet::weight(1_000_000)]
		pub fn remove_order(
			origin: OriginFor<T>,
			cid: RootId,
		) -> DispatchResult {
			todo!()
		}
	}
}


impl<T: Config> Pallet<T> {
	pub fn update_nodes() {

	}
}
