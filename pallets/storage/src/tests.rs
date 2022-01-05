use super::*;

use crate::mock::*;

use frame_support::{assert_err, assert_ok, traits::Currency};

use pallet::Event as PalletEvent;

const MB: u64 = 1_048_576;
const MB2: u128 = 1_048_576;

macro_rules! assert_node {
    ($x:expr, $($k:ident : $v:expr),+ $(,)?) => {
        {
            let node_info = Nodes::<Test>::get($x).unwrap();
            $(assert_eq!(node_info.$k, $v);)+
        }
    };
}

macro_rules! assert_file {
    ($x:expr, $($k:ident : $v:expr),+ $(,)?) => {
        {
            let file_info = Files::<Test>::get($x).unwrap();
            $(assert_eq!(file_info.$k, $v);)+
        }
    };
}

macro_rules! assert_summary {
    ($x:expr, $($k:ident : $v:expr),+ $(,)?) => {
        {
            let summary = Summarys::<Test>::get($x);
            $(assert_eq!(summary.$k, $v);)+
        }
    };
}

macro_rules! assert_session {
    ($($k:ident : $v:expr),+ $(,)?) => {
        {
            let session = Session::<Test>::get();
            $(assert_eq!(session.$k, $v);)+
        }
    };
}

macro_rules! assert_last_pallet_event {
	($e:expr) => {
		assert_eq!(
			frame_system::Pallet::<Test>::events()
				.pop()
				.map(|e| e.event)
				.expect("Event expected"),
			mock::Event::FileStorage($e).into()
		);
	};
}

#[test]
fn set_enclave_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(FileStorage::set_enclave(Origin::root(), MACHINES[1].get_enclave(), 100));

		// should shorten period
		assert_ok!(FileStorage::set_enclave(Origin::root(), MACHINES[0].get_enclave(), 100));

		// should not growth period
		assert_err!(
			FileStorage::set_enclave(Origin::root(), MACHINES[0].get_enclave(), 100),
			Error::<Test>::EnclaveExpired
		);
	});
}

#[test]
fn stash_works() {
	ExtBuilder::default().build().execute_with(|| {
		let stash_balance = default_stash_balance();
		let u1 = Balances::free_balance(1);
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		assert_eq!(Balances::free_balance(1), u1.saturating_sub(stash_balance));
		assert_eq!(balance_of_storage_pot(), stash_balance + 1);
		assert_node!(2, deposit: stash_balance);

		// should recharge when account's stash_balance < T::StashBalance
		let stash_balance_mul_3 = stash_balance.saturating_mul(3);
		change_stash_balance(stash_balance_mul_3);
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		assert_eq!(Balances::free_balance(1), u1.saturating_sub(stash_balance_mul_3));
		assert_node!(2, deposit: stash_balance_mul_3);
		assert_eq!(balance_of_storage_pot(), stash_balance_mul_3 + 1);

		// should do nothing when account's stash_balance > T::StashBalance
		change_stash_balance(stash_balance);
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		assert_eq!(Balances::free_balance(1), u1.saturating_sub(stash_balance_mul_3));
		assert_node!(2, deposit: stash_balance_mul_3);
		assert_eq!(balance_of_storage_pot(), stash_balance_mul_3 + 1);

		// one stasher multiple controller
		assert_ok!(FileStorage::stash(Origin::signed(1), 3));

		// should not stash another controller
		assert_err!(FileStorage::stash(Origin::signed(11), 2), Error::<Test>::NotPair,);
	})
}

#[test]
fn stash_coverd_used_deposit() {
	ExtBuilder::default().build().execute_with(|| {
		let u1 = Balances::free_balance(1);
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));

		Nodes::<Test>::mutate(2, |maybe_node_info| {
			if let Some(node_info) = maybe_node_info {
				node_info.used = node_info.used.saturating_add(MB);
			}
		});
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		assert_eq!(
			Balances::free_balance(1),
			u1.saturating_sub(default_stash_balance()).saturating_sub(10)
		);
	})
}

#[test]
fn withdraw_works() {
	ExtBuilder::default().build().execute_with(|| {
		Balances::make_free_balance_be(&FileStorage::account_id(), 2_000);

		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		Nodes::<Test>::mutate(2, |maybe_node_info| {
			if let Some(node_info) = maybe_node_info {
				node_info.deposit = node_info.deposit.saturating_add(1_000);
			}
		});
		let pot = balance_of_storage_pot();
		let u1 = Balances::free_balance(1);
		assert_ok!(FileStorage::withdraw(Origin::signed(2)));
		assert_node!(2, deposit: default_stash_balance());
		assert_eq!(Balances::free_balance(1), u1.saturating_add(1_000));
		assert_eq!(balance_of_storage_pot(), pot.saturating_sub(1_000));

		// should not withdraw when account's deposit < T::StashBalance
		assert_ok!(FileStorage::stash(Origin::signed(11), 12));
		assert_err!(FileStorage::withdraw(Origin::signed(12)), Error::<Test>::NoEnoughToWithdraw,);
	})
}

#[test]
fn withdraw_reserve_used_deposit() {
	ExtBuilder::default().build().execute_with(|| {
		Balances::make_free_balance_be(&FileStorage::account_id(), 2_000);

		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		Nodes::<Test>::mutate(2, |maybe_node_info| {
			if let Some(node_info) = maybe_node_info {
				node_info.deposit = node_info.deposit.saturating_add(1_000);
				node_info.used = node_info.used.saturating_add(MB);
			}
		});
		assert_ok!(FileStorage::withdraw(Origin::signed(2)));
		assert_eq!(Nodes::<Test>::get(2).unwrap().deposit, default_stash_balance() + 10);
	})
}

#[test]
fn register_works() {
	ExtBuilder::default().stash(1, 2).build().execute_with(|| {
		let register_data = MACHINES[0].register_data();
		let machine_id = register_data.machine_id.clone();
		assert_node!(2, machine_id: None);
		assert_ok!(register_data.call(2));
		assert_node!(2, machine_id: Some(machine_id.clone()));
		assert_eq!(Registers::<Test>::get(&machine_id).unwrap(), MACHINES[0].register_info());

		// register again with different register info
		assert_ok!(MACHINES[1].register(2));
		assert_eq!(Registers::<Test>::get(&machine_id).unwrap(), MACHINES[1].register_info());

		// Failed when controller is not stashed
		assert_err!(MACHINES[0].register(3), Error::<Test>::NodeNotStashed);

		// Failed when machind_id don't match
		let mut register_data = MACHINES[0].register_data();
		register_data.machine_id[0] += 1;
		assert_err!(register_data.call(2), Error::<Test>::MismatchMacheId);

		// Failed when enclave is not inclued
		assert_err!(MACHINES[2].register(2), Error::<Test>::InvalidEnclave);

		// Failed when relady registered machine
		assert_ok!(FileStorage::stash(Origin::signed(1), 3));
		assert_err!(MACHINES[1].register(3), Error::<Test>::MachineAlreadyRegistered);
	})
}

#[test]
fn report_works() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, MACHINES[0].register_data())
		.files(vec![(mock_file_id('A'), MB, 2000)])
		.build()
		.execute_with(|| {
			let now_at = FileStorage::now_at();
			assert_session! { current: 0 };
			let report_data = MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(0);
			assert_ok!(report_data.call(2));
			assert_last_pallet_event!(PalletEvent::NodeReported {
				controller: 2,
				machine_id: get_machine_id(0),
				mine_reward: 0,
				share_store_reward: 0,
				direct_store_reward: 0,
				slash: 0,
			});
			assert_file!(mock_file_id('A'),
				base_fee: 0,
				file_size: MB,
				reserved: 900,
				added_at: now_at,
				fee: 100,
				file_size: MB,
				expire_at: 31,
				replicas: vec![2],
			);
			assert_eq!(StoragePotReserved::<Test>::get(), 1000);
			assert_summary!(0,
				count: 1,
				power: 10 * MB2,
				used: MB2,
			);
			assert_node!(2,
			deposit: default_stash_balance(), rid: 3, reported_at: now_at,
				power: 10 * MB, used: MB, slash_used: 0,
				power: 10 * MB,
				used: MB,
			);

			// Failed when report twice in same session
			assert_err!(report_data.call(2), Error::<Test>::DuplicateReport);
		})
}

#[test]
fn report_works_with_useless_files() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, MACHINES[0].register_data())
		.build()
		.execute_with(|| {
			assert_ok!(MockData::new(0, 3, 10 * MB, &[('A', MB)])
				.del_files(&['B'])
				.settle_files(&['C'])
				.report_data(0)
				.call(2));

			assert_node!(2, rid: 3, used: 0);
		})
}

#[test]
fn file_order_removed_if_file_size_is_too_small() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, MACHINES[0].register_data())
		.files(vec![(mock_file_id('A'), 100, 1100)])
		.build()
		.execute_with(|| {
			assert_ok!(MockData::new(0, 3, 10 * MB, &[('A', 2 * MB)]).report_data(0).call(2));
			assert_last_pallet_event!(PalletEvent::NodeReported {
				controller: 2,
				machine_id: get_machine_id(0),
				mine_reward: 0,
				share_store_reward: 0,
				direct_store_reward: 10,
				slash: 0,
			});
			assert_node!(2, deposit: default_stash_balance() + 10);
			assert_summary!(0,
				store_reward: 90,
			);
			assert_eq!(Files::<Test>::get(&mock_file_id('A')), None);
		})
}

#[test]
fn file_order_do_not_add_replica_when_exceed_max_replicas() {
	let report_data = MockData::new(0, 3, 200, &[('A', 100)]).report_data(3);
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, MACHINES[0].register_data())
		.files(vec![(mock_file_id('A'), MB, 1100)])
		.reports(vec![
			(6, MACHINES[3].register_data(), report_data.clone()),
			(7, MACHINES[3].register_data(), report_data.clone()),
			(8, MACHINES[3].register_data(), report_data.clone()),
			(9, MACHINES[3].register_data(), report_data.clone()),
			(10, MACHINES[3].register_data(), report_data.clone()),
		])
		.build()
		.execute_with(|| {
			assert_ok!(MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(0).call(2));
			assert_file!(mock_file_id('A'), replicas: vec![6, 7, 8, 9, 10]);
			assert_node!(2,  power: 10 * MB, used: 0 );
		})
}

#[test]
fn file_order_remove_replica_if_not_report() {
	let report_data = MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(3);
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, MACHINES[0].register_data())
		.files(vec![(mock_file_id('A'), MB, 1100)])
		.reports(vec![
			(8, MACHINES[3].register_data(), report_data.clone()),
			(9, MACHINES[3].register_data(), report_data.clone()),
		])
		.build()
		.execute_with(|| {
			run_to_block(11);
			assert_ok!(MockData::new(3, 4, 10 * MB, &[]).report_data(3).call(9));

			run_to_block(21);
			assert_ok!(MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(0).call(2));
			assert_file!(mock_file_id('A'), replicas: vec![9, 2]);
			assert_node!(8, rid: 3, power: 10 * MB, used: 0, slash_used: MB);
		})
}

#[test]
fn report_del_files() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, MACHINES[0].register_data())
		.files(vec![(mock_file_id('A'), MB, 1100)])
		.build()
		.execute_with(|| {
			assert_ok!(MACHINES[0].register(2));
			assert_ok!(MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(0).call(2));
			assert_file!(mock_file_id('A'), replicas: vec![2]);
			assert_node!(2, rid: 3, reported_at: 1, power: 10 * MB, used: MB, slash_used: 0);
			run_to_block(11);
			assert_ok!(MockData::new(3, 5, 9 * MB, &vec![])
				.del_files(&['A'])
				.report_data(0)
				.call(2));
			assert_file!(mock_file_id('A'), replicas: Vec::<AccountId>::new());
			assert_node!(2,
				deposit: default_stash_balance() - 10,
				rid: 5,
				reported_at: 11,
				power: 9 * MB,
				used: 0,
				slash_used: 0,
				reward: 0,
				prev_reported_at: 1,
			);
		})
}

#[test]
fn report_settle_files() {
	let report_data = MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(0);
	ExtBuilder::default()
		.files(vec![(mock_file_id('A'), MB, 1100)])
		.reports(vec![(2, MACHINES[0].register_data(), report_data.clone())])
		.build()
		.execute_with(|| {
			assert_node!(2, deposit: default_stash_balance(), reported_at: 1, prev_reported_at: 0);
			assert_file!(mock_file_id('A'), expire_at: 31);
			run_to_block(11);
			assert_ok!(MockData::new(3, 4, 10 * MB, &[]).report_data(0).call(2));
			assert_node!(2, reported_at: 11, prev_reported_at: 1);
			run_to_block(21);
			assert_ok!(MockData::new(4, 5, 10 * MB, &[]).report_data(0).call(2));
			assert_node!(2, reported_at: 21, prev_reported_at: 11);
			run_to_block(31);
			assert_ok!(MockData::new(5, 6, 9 * MB, &[])
				.settle_files(&['A'])
				.report_data(0)
				.call(2));
			assert_node!(2, power: 9 * MB, used: 0, deposit: default_stash_balance() + 20);
			assert_last_pallet_event!(PalletEvent::NodeReported {
				controller: 2,
				machine_id: get_machine_id(0),
				mine_reward: 0,
				share_store_reward: 0,
				direct_store_reward: 20,
				slash: 0,
			});
			assert_session! { current: 3 };
			assert_summary!(3, store_reward: 80);
			assert_eq!(Files::<Test>::get(&mock_file_id('A')), None);
		})
}

#[test]
fn report_settle_files_do_not_reward_unhealth_node() {
	ExtBuilder::default()
		.files(vec![(mock_file_id('A'), MB, 1200)])
		.reports(vec![
			(
				2,
				MACHINES[0].register_data(),
				MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(0).clone(),
			),
			(
				3,
				MACHINES[3].register_data(),
				MockData::new(0, 3, 100 * MB, &[('A', MB)]).report_data(3).clone(),
			),
		])
		.build()
		.execute_with(|| {
			run_to_block(11);
			assert_ok!(MockData::new(3, 4, 10 * MB, &[]).report_data(0).call(2));
			assert_ok!(MockData::new(3, 4, 10 * MB, &[]).report_data(3).call(3));
			run_to_block(21);
			assert_ok!(MockData::new(4, 5, 10 * MB, &[]).report_data(3).call(3));
			run_to_block(31);
			assert_ok!(MockData::new(5, 6, 10 * MB, &[])
				.settle_files(&['A'])
				.report_data(3)
				.call(3));
			assert_file!(mock_file_id('A'), replicas: vec![3]);
			assert_node!(3, deposit: default_stash_balance() + 20);
			assert_node!(2, deposit: default_stash_balance(), rid: 4, 
            reported_at: 11, power: 10 * MB, used: 0, slash_used: MB);
			assert_ok!(MockData::new(4, 5, 10 * MB, &[]).report_data(0).call(2));
			assert_last_pallet_event!(PalletEvent::NodeReported {
				controller: 2,
				machine_id: get_machine_id(0),
				mine_reward: 0,
				share_store_reward: 0,
				direct_store_reward: 0,
				slash: 110,
			});
			assert_node!(2, deposit: default_stash_balance() - 110, rid: 5,
            reported_at: 31, power: 10 * MB, used: 0, slash_used: 0);
		})
}

#[test]
fn report_do_store_reward() {
	ExtBuilder::default()
		.files(vec![(mock_file_id('A'), MB, 1200)])
		.reports(vec![
			(
				2,
				MACHINES[0].register_data(),
				MockData::new(0, 3, 100 * MB, &[('A', MB)]).report_data(0).clone(),
			),
			(
				3,
				MACHINES[3].register_data(),
				MockData::new(0, 3, 100 * MB, &[('A', MB)]).report_data(3).clone(),
			),
		])
		.build()
		.execute_with(|| {
			Files::<Test>::mutate(mock_file_id('A'), |maybe_file| {
				if let Some(file) = maybe_file {
					file.expire_at = 11;
				}
			});
			run_to_block(11);
			assert_ok!(MockData::new(3, 4, 10 * MB, &[])
				.settle_files(&['A'])
				.report_data(0)
				.call(2));
			assert_last_pallet_event!(PalletEvent::NodeReported {
				controller: 2,
				machine_id: get_machine_id(0),
				mine_reward: 0,
				share_store_reward: 0,
				direct_store_reward: 20,
				slash: 0,
			});
			assert_node!(2, deposit: default_stash_balance() + 20);
			assert_node!(3, deposit: default_stash_balance(), reward: 10);
			assert_ok!(MockData::new(3, 4, 10 * MB, &[]).report_data(3).call(3));
			assert_node!(3, deposit: default_stash_balance() + 10, reward: 0);
			assert_session! { current: 1 };
			assert_summary!(1,
				mine_reward: 0,
				store_reward: 70,
				paid_mine_reward: 0,
				paid_store_reward: 0,
			);
			run_to_block(21);
			assert_ok!(MockData::new(4, 5, 10 * MB, &[]).report_data(0).call(2));
			assert_last_pallet_event!(PalletEvent::NodeReported {
				controller: 2,
				machine_id: get_machine_id(0),
				mine_reward: 0,
				share_store_reward: 35,
				direct_store_reward: 0,
				slash: 0,
			});
			assert_ok!(MockData::new(4, 5, 10 * MB, &[]).report_data(3).call(3));
			assert_last_pallet_event!(PalletEvent::NodeReported {
				controller: 3,
				machine_id: get_machine_id(3),
				mine_reward: 0,
				share_store_reward: 35,
				direct_store_reward: 0,
				slash: 0,
			});
		})
}

#[test]
fn report_do_mine_reward() {
	ExtBuilder::default()
		.files(vec![(mock_file_id('A'), MB, 1200)])
		.reports(vec![
			(
				2,
				MACHINES[0].register_data(),
				MockData::new(0, 3, 100 * MB, &[('A', MB)]).report_data(0).clone(),
			),
			(
				3,
				MACHINES[3].register_data(),
				MockData::new(0, 3, 100 * MB, &[('A', MB)]).report_data(3).clone(),
			),
		])
		.mine_factor(Perbill::from_percent(1))
		.build()
		.execute_with(|| {
			run_to_block(11);
			assert_session! { current: 1 };
			assert_summary!(0,
				mine_reward: 2 * MB2,
				store_reward: 0,
				paid_mine_reward: 0,
				paid_store_reward: 0,
			);
			assert_ok!(MockData::new(3, 4, 10, &[]).report_data(0).call(2));
			assert_last_pallet_event!(PalletEvent::NodeReported {
				controller: 2,
				machine_id: get_machine_id(0),
				mine_reward: MB.saturated_into(),
				share_store_reward: 0,
				direct_store_reward: 0,
				slash: 0,
			});
			assert_ok!(MockData::new(3, 4, 10, &[]).report_data(3).call(3));
			assert_last_pallet_event!(PalletEvent::NodeReported {
				controller: 3,
				machine_id: get_machine_id(3),
				mine_reward: MB.saturated_into(),
				share_store_reward: 0,
				direct_store_reward: 0,
				slash: 0,
			});
		})
}

#[test]
fn report_slash() {
	let report_data = MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(0);
	ExtBuilder::default()
		.files(vec![(mock_file_id('A'), MB, 1200)])
		.reports(vec![(2, MACHINES[0].register_data(), report_data.clone())])
		.build()
		.execute_with(|| {
			run_to_block(11);
			assert_ok!(MockData::new(3, 4, 10 * MB, &[]).report_data(0).call(2));
			run_to_block(21);
			let pot_reserved = StoragePotReserved::<Test>::get();
			run_to_block(31);
			assert_ok!(MockData::new(4, 5, 10 * MB, &[])
				.settle_files(&['A'])
				.report_data(0)
				.call(2));
			assert_last_pallet_event!(PalletEvent::NodeReported {
				controller: 2,
				machine_id: get_machine_id(0),
				mine_reward: 0,
				share_store_reward: 0,
				direct_store_reward: 0,
				slash: 110,
			});

			assert_node!(2, deposit: default_stash_balance() - 110);
			assert_eq!(StoragePotReserved::<Test>::get(), pot_reserved.saturating_add(110));
			assert_eq!(Files::<Test>::get(&mock_file_id('A')).unwrap().replicas.len(), 0);
		})
}

#[test]
fn report_failed_with_legal_input() {
	ExtBuilder::default().build().execute_with(|| {
		// Failed when controller is not stashed
		let report_data = MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(0);
		assert_err!(report_data.call(2), Error::<Test>::NodeNotStashed);

		// Failed when controller is not registered
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		assert_err!(report_data.call(2), Error::<Test>::UnregisterNode);

		assert_ok!(MACHINES[0].register(2));

		// Failed when add_files or del_files is tampered
		let mut report_data2 = report_data.clone();
		report_data2.add_files[0].1 = report_data2.add_files[0].1 + 1;
		assert_err!(report_data2.call(2), Error::<Test>::InvalidVerifyP256Sig);

		// Failed when enclave is outdated
		run_to_block(1001);
		assert_err!(report_data.call(2), Error::<Test>::InvalidEnclave);
	})
}

#[test]
fn report_failed_when_rid_is_not_continuous() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, MACHINES[0].register_data())
		.files(vec![(mock_file_id('A'), MB, 1100)])
		.build()
		.execute_with(|| {
			assert_ok!(MACHINES[0].register(2));
			let report_data = MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(0);
			assert_ok!(report_data.call(2));
			run_to_block(11);
			assert_err!(report_data.call(2), Error::<Test>::InvalidVerifyP256Sig);
			Nodes::<Test>::mutate(2, |maybe_node_info| {
				if let Some(node_info) = maybe_node_info {
					node_info.rid = 0;
				}
			});
			assert_ok!(report_data.call(2));
		})
}

#[test]
fn store_works() {
	ExtBuilder::default().build().execute_with(|| {
		let pot = balance_of_storage_pot();
		let u1000 = Balances::free_balance(&1000);
		let file_fee = 1100;
		assert_eq!(FileStorage::store_file_fee(2000), file_fee);
		assert_ok!(FileStorage::store(Origin::signed(1000), mock_file_id('A'), MB, file_fee));
		let now_at = FileStorage::now_at();
		assert_file!(mock_file_id('A'),
			reserved: file_fee.saturating_sub(FILE_BASE_PRICE),
			base_fee: FILE_BASE_PRICE,
			file_size: MB,
			added_at: now_at,
		);
		assert_eq!(Balances::free_balance(&1000), u1000 - file_fee);
		assert_eq!(balance_of_storage_pot(), pot.saturating_add(file_fee));

		// Add more fee
		assert_ok!(FileStorage::store(Origin::signed(1000), mock_file_id('A'), MB, 10,));
		assert_file!(mock_file_id('A'),
			reserved: file_fee.saturating_sub(FILE_BASE_PRICE).saturating_add(10),
			base_fee: FILE_BASE_PRICE,
			file_size: MB,
			added_at: now_at,
		);

		// Failed when fee is not enough
		assert_err!(
			FileStorage::store(
				Origin::signed(100),
				mock_file_id('B'),
				MB,
				file_fee.saturating_sub(1),
			),
			Error::<Test>::NotEnoughFee
		);

		// Failed when fize size not in [0, T::MaxFileSize]
		assert_err!(
			FileStorage::store(
				Origin::signed(1000),
				mock_file_id('X'),
				MAX_FILE_SIZE + 1,
				u128::max_value(),
			),
			Error::<Test>::InvalidFileSize
		);
	})
}

#[test]
fn force_delete() {
	ExtBuilder::default()
		.files(vec![(mock_file_id('A'), 100, 1100)])
		.build()
		.execute_with(|| {
			run_to_block(31);
			assert_err!(
				FileStorage::force_delete(Origin::root(), mock_file_id('A')),
				Error::<Test>::UnableToDeleteFile
			);
			run_to_block(32);
			assert_eq!(StoragePotReserved::<Test>::get(), 0);
			assert_ok!(FileStorage::force_delete(Origin::root(), mock_file_id('A')));
			assert_eq!(Files::<Test>::get(&mock_file_id('A')), None);
			assert_eq!(StoragePotReserved::<Test>::get(), 1100);
		})
}

#[test]
fn calculate_mine() {
	ExtBuilder::default()
		.mine_factor(Perbill::from_percent(1))
		.build()
		.execute_with(|| {
			run_to_block(11);
			assert_session! { current: 1 };
			assert_eq!(StoragePotReserved::<Test>::get(), 0);
			assert_eq!(FileStorage::calculate_mine(1), (0, 0, 0));
			Summarys::<Test>::insert(
				1,
				SummaryInfo { power: 400 * MB2, used: 0, ..Default::default() },
			);
			assert_eq!(FileStorage::calculate_mine(1), (4 * MB2, 0, 4 * MB2));
			StoragePotReserved::<Test>::set(MB2);
			assert_eq!(FileStorage::calculate_mine(1), (4 * MB2, 0, 3 * MB2));
			Summarys::<Test>::insert(
				0,
				SummaryInfo {
					mine_reward: 2 * MB2,
					store_reward: 2 * MB2,
					paid_mine_reward: MB2,
					paid_store_reward: MB2,
					..Default::default()
				},
			);
			assert_eq!(FileStorage::calculate_mine(1), (4 * MB2, 0, 1 * MB2));
		})
}

#[test]
fn session_end() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			Session::<Test>::get(),
			SessionState { current: 0, prev_begin_at: 0, begin_at: 1, end_at: 10 }
		);
		run_to_block(10);
		assert_eq!(
			Session::<Test>::get(),
			SessionState { current: 0, prev_begin_at: 0, begin_at: 1, end_at: 10 }
		);
		run_to_block(11);
		assert_eq!(
			Session::<Test>::get(),
			SessionState { current: 1, prev_begin_at: 1, begin_at: 11, end_at: 20 }
		);
	})
}

#[test]
fn session_end2() {
	ExtBuilder::default()
		.mine_factor(Perbill::from_percent(1))
		.build()
		.execute_with(|| {
			run_to_block(11);
			assert_session! { current: 1 };
			Summarys::<Test>::insert(
				1,
				SummaryInfo { power: 400 * MB2, used: 0, ..Default::default() },
			);
			StoragePotReserved::<Test>::set(MB2);
			Summarys::<Test>::insert(
				0,
				SummaryInfo {
					mine_reward: 2 * MB2,
					store_reward: 2 * MB2,
					paid_mine_reward: MB2,
					paid_store_reward: MB2,
					..Default::default()
				},
			);
			let pb = Balances::free_balance(&FileStorage::account_id());
			StoragePotReserved::<Test>::set(MB2);
			run_to_block(21);
			assert_eq!(Balances::free_balance(&FileStorage::account_id()), pb.saturating_add(MB2));
			assert_summary!(1,
				mine_reward: 4 * MB2,
				store_reward: 0,
				paid_mine_reward: 0,
				paid_store_reward: 0,
			);
			assert_last_pallet_event!(PalletEvent::NewSession { index: 1, mine: MB2 });
		})
}

#[test]
fn session_end_clear_prev_prev_summary() {
	ExtBuilder::default().build().execute_with(|| {
		assert_session! { current: 0 };
		Summarys::<Test>::insert(0, SummaryInfo { power: 400 * MB2, ..Default::default() });
		run_to_block(11);
		assert_summary! { 0, power: 400 * MB2 };
		run_to_block(21);
		assert_summary! { 0, power: 400 * MB2 };
		run_to_block(31);
		assert_summary! { 0, power: 0 };
	})
}

#[test]
fn store_fee_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(FileStorage::store_fee(MB, 30), 1100);
		assert_eq!(FileStorage::store_fee(MB, 10), 1100);
		assert_eq!(FileStorage::store_fee(100, 10), 1100);
		assert_eq!(FileStorage::store_fee(MB, 31), 1200);
	})
}

#[test]
fn node_deposit_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			FileStorage::node_deposit(&2),
			NodeDepositInfo {
				current_deposit: 0,
				slash_deposit: default_stash_balance(),
				used_deposit: 0
			}
		);
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		assert_eq!(
			FileStorage::node_deposit(&2),
			NodeDepositInfo {
				current_deposit: default_stash_balance(),
				slash_deposit: default_stash_balance(),
				used_deposit: 0
			}
		);
		assert_ok!(MACHINES[0].register(2));
		Nodes::<Test>::mutate(2, |maybe_node_info| {
			if let Some(node_info) = maybe_node_info {
				node_info.used = node_info.used.saturating_add(MB);
			}
		});
		assert_eq!(
			FileStorage::node_deposit(&2),
			NodeDepositInfo {
				current_deposit: default_stash_balance(),
				slash_deposit: default_stash_balance(),
				used_deposit: 10
			}
		);
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		assert_eq!(
			FileStorage::node_deposit(&2),
			NodeDepositInfo {
				current_deposit: default_stash_balance() + 10,
				slash_deposit: default_stash_balance(),
				used_deposit: 10
			}
		);
	})
}
