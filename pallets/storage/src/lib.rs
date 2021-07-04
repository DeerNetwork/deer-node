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
	ensure, BoundedVec,
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

#[derive(Clone, Encode, Decode, Eq, PartialEq, Default, RuntimeDebug)]
pub struct StorageStats {
	pub used: u128,
	pub free: u128,
	pub files_size: u128,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct OrderInfo<Balance, BlockNumber> {
    pub cid: RootId,
    pub file_size: u64,
    pub duration: BlockNumber,
    pub reserve: Balance,
}
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct FileInfo<AccountId, BlockNumber> {
    pub file_size: u64,
    pub orders: Vec<AccountId>,
    // pub file_amount: Balance,
    pub expire: BlockNumber,
    pub replicas: Vec<AccountId>,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct StashInfo<AccountId, Balance> {
    pub stash: AccountId,
    pub amount: Balance,
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

		type Currency: ReservableCurrency<Self::AccountId>;

		#[pallet::constant]
		type SlashDeferDuration: Get<ReportRound>;

		#[pallet::constant]
		type RoundDuration: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type RoundWindowSize: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type FileDuration: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type MaxFileReplica: Get<u32>;

		#[pallet::constant]
		type FilePrice: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxTrashSize: Get<u128>;

		#[pallet::constant]
		type MaxFileSize: Get<u64>;

		#[pallet::constant]
		type MinStashBalance: Get<BalanceOf<Self>>;
	}

	#[pallet::type_value]
	pub fn RoundOnEmpty() -> ReportRound { 0 }

	#[pallet::type_value]
	pub fn StorageStatsOnEmpty() -> StorageStats {
		Default::default()
	}

	#[pallet::storage]
	#[pallet::getter(fn enclaves)]
	pub type Enclaves<T: Config> = StorageMap<
		_,
		Twox64Concat,
		EnclaveId,
		BlockNumberFor<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn nodes)]
	pub type Nodes<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		NodeInfo,
	>;

	#[pallet::storage]
	#[pallet::getter(fn registers)]
	pub type Registers<T: Config> = StorageMap<
		_,
		Twox64Concat,
		PubKey,
		RegisterInfo,
	>;

	#[pallet::storage]
	#[pallet::getter(fn reporters)]
	pub type Reporters<T: Config> = StorageMap<
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
	#[pallet::getter(fn round_reports)]
	pub type RoundReports<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat, PubKey,
		Twox64Concat, ReportRound,
		bool,
	>;

	#[pallet::storage]
	#[pallet::getter(fn orders)]
	pub type Orders<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat, T::AccountId,
		Twox64Concat, RootId,
		OrderInfo<BalanceOf<T>, T::BlockNumber>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn files)]
	pub type Files<T: Config> = StorageMap<
		_,
		Twox64Concat, RootId,
		FileInfo<T::AccountId, T::BlockNumber>,
	>;


	#[pallet::storage]
	#[pallet::getter(fn replicas)]
	pub type Replicas<T: Config> = StorageMap<
		_,
		Twox64Concat, RootId,
		FileInfo<T::AccountId, T::BlockNumber>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn stashs)]
	pub type Stashs<T: Config> = StorageMap<
		_,
		Blake2_128Concat, T::AccountId,
		StashInfo<T::AccountId, BalanceOf<T>>,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
	pub enum Event<T: Config> {
        SetEnclave(EnclaveId, T::BlockNumber)
	}

	#[pallet::error]
	pub enum Error<T> {
        InvalidEnclaveExpire,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> frame_support::weights::Weight {
			if (now % T::RoundDuration::get()).is_zero() {
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
            ensure_root(origin)?;
            if let Some(old_expire) = Self::enclaves(&enclave) {
                ensure!(expire < old_expire, Error::<T>::InvalidEnclaveExpire);
            }
            Enclaves::<T>::insert(&enclave, &expire);
            Self::deposit_event(Event::<T>::SetEnclave(enclave, expire));

            Ok(())
		}


		#[pallet::weight(1_000_000)]
		pub fn set_stash(
			origin: OriginFor<T>,
			controller: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			todo!()
		}

		#[pallet::weight(1_000_000)]
		pub fn register_node(
			origin: OriginFor<T>,
			cert: Vec<u8>,
			body: Vec<u8>,
			sig: Vec<u8>,
			checksum: Vec<u8>,
		) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let curr_bn = <frame_system::Pallet<T>>::block_number();
            let legal_enclaves: Vec<EnclaveId> = <Enclaves<T>>::iter()
                .filter(|(_, bn)| bn > &curr_bn)
                .map(|(v, _)| v)
                .collect();
            
            let applier = who.encode();

            Ok(())
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

		#[pallet::weight(0)]
		pub fn report_offline(
			origin: OriginFor<T>,
			stash: T::AccountId,
			at: T::BlockNumber
		) -> DispatchResult {
			todo!()
		}

		#[pallet::weight(1_000_000)]
		pub fn save_file(
			origin: OriginFor<T>,
			cid: RootId,
			file_size: u64,
			duration: T::BlockNumber,
			reserve: BalanceOf<T>,
		) -> DispatchResult {
			todo!()
		}

		#[pallet::weight(1_000_000)]
		pub fn unsave_file(
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

