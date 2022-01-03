use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_ok, traits::Currency};

use pallet::Event as PalletEvent;

const MB: u64 = 1_048_576;
const MB2: u128 = 1_048_576;

macro_rules! assert_node_info {
    ($x:expr, $($k:ident => $v:expr),+ $(,)?) => {
        {
            let node_info = Nodes::<Test>::get($x).unwrap();
            $(assert_eq!(node_info.$k, $v);)+
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
		assert_node_info!(2, deposit => stash_balance);

		// should recharge when account's stash_balance < T::StashBalance
		let stash_balance_mul_3 = stash_balance.saturating_mul(3);
		change_stash_balance(stash_balance_mul_3);
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		assert_eq!(Balances::free_balance(1), u1.saturating_sub(stash_balance_mul_3));
		assert_node_info!(2, deposit => stash_balance_mul_3);
		assert_eq!(balance_of_storage_pot(), stash_balance_mul_3 + 1);

		// should do nothing when account's stash_balance > T::StashBalance
		change_stash_balance(stash_balance);
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		assert_eq!(Balances::free_balance(1), u1.saturating_sub(stash_balance_mul_3));
		assert_node_info!(2, deposit => stash_balance_mul_3);
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
		assert_node_info!(2, deposit => default_stash_balance());
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
		assert_node_info!(2, machine_id => None);
		assert_ok!(register_data.call(2));
		assert_node_info!(2, machine_id => Some(machine_id.clone()));
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
			let current_round = CurrentRound::<Test>::get();
			let report_data = MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(0);
			assert_ok!(report_data.call(2));
            assert_last_pallet_event!(crate::Event::NodeReported {
				controller: 2,
				machine_id: MACHINES[0].get_machine_id(),
				mine_reward: 0,
				share_store_reward: 0,
				direct_store_reward: 0,
				slash: 0,
			});
			assert_eq!(
				StoreFiles::<Test>::get(&mock_file_id('A')).unwrap(),
				StoreFile { base_fee: 0, file_size: MB, reserved: 900, added_at: now_at }
			);
			assert_eq!(
				FileOrders::<Test>::get(&mock_file_id('A')).unwrap(),
				FileOrder { fee: 100, file_size: MB, expire_at: 31, replicas: vec![2] }
			);
			assert_eq!(StoragePotReserved::<Test>::get(), 1000);
			assert_eq!(
				RoundsReward::<Test>::get(current_round),
				RewardInfo {
					mine_reward: 0,
					store_reward: 0,
					paid_mine_reward: 0,
					paid_store_reward: 0,
				}
			);
			assert_eq!(
				RoundsReport::<Test>::get(current_round, 2).unwrap(),
				NodeStats { power: 10 * MB, used: MB }
			);
			assert_eq!(
				RoundsSummary::<Test>::get(current_round),
				SummaryStats { power: 10 * MB2, used: MB2 }
			);

            assert_node_info!(2, deposit => default_stash_balance(), rid => 3, reported_at => now_at, power => 10 * MB, used => MB, slash_used => 0);

			// Failed when report twice in same round
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

			assert_node_info!(2, rid => 3, used => 0);
		})
}

#[test]
fn file_order_removed_if_file_size_is_small_than_actual_and_is_lack_fee() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, MACHINES[0].register_data())
		.files(vec![(mock_file_id('A'), 100, 1100)])
		.build()
		.execute_with(|| {
			assert_ok!(MockData::new(0, 3, 10 * MB, &[('A', 2 * MB)]).report_data(0).call(2));
			assert_last_pallet_event!(crate::Event::NodeReported {
				controller: 2,
				machine_id: MACHINES[0].get_machine_id(),
				mine_reward: 0,
				share_store_reward: 0,
				direct_store_reward: 10,
				slash: 0,
			});
			assert_node_info!(2, deposit => default_stash_balance() + 10);
			assert_eq!(RoundsReward::<Test>::get(CurrentRound::<Test>::get()).store_reward, 90);
			assert_eq!(StoreFiles::<Test>::get(&mock_file_id('A')), None);
			assert_eq!(FileOrders::<Test>::get(&mock_file_id('A')), None);
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
			assert_eq!(
				FileOrders::<Test>::get(&mock_file_id('A')).unwrap().replicas,
				vec![6, 7, 8, 9, 10]
			);
			assert_eq!(
				RoundsReport::<Test>::get(CurrentRound::<Test>::get(), 2).unwrap(),
				NodeStats { power: 10 * MB, used: 0 }
			);
		})
}

#[test]
fn file_order_remove_replica_if_node_fail_to_report() {
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
			assert_eq!(FileOrders::<Test>::get(&mock_file_id('A')).unwrap().replicas, vec![9, 2]);
			assert_node_info!(8, rid => 3, power => 10 * MB, used => 0, slash_used => MB);
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
			assert_eq!(FileOrders::<Test>::get(&mock_file_id('A')).unwrap().replicas, vec![2]);
            assert_node_info!(2, rid => 3, reported_at => 1, power => 10 * MB, used => MB, slash_used => 0);
			run_to_block(11);
			assert_ok!(MockData::new(3, 5, 9 * MB, &vec![])
				.del_files(&['A'])
				.report_data(0)
				.call(2));
			assert_eq!(FileOrders::<Test>::get(&mock_file_id('A')).unwrap().replicas.len(), 0);
            assert_node_info!(2, deposit => default_stash_balance() - 10, rid => 5, reported_at => 11, power => 9 * MB, used => 0, slash_used => 0);
			assert_eq!(
				RoundsReport::<Test>::get(CurrentRound::<Test>::get(), 2).unwrap(),
				NodeStats { power: 9 * MB, used: 0 }
			);
		})
}

#[test]
fn report_settle_files() {
	let report_data = MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(3);
	ExtBuilder::default()
		.files(vec![(mock_file_id('A'), MB, 1100)])
		.reports(vec![(2, MACHINES[3].register_data(), report_data.clone())])
		.build()
		.execute_with(|| {
			assert_node_info!(2, deposit => default_stash_balance());
			assert_eq!(RoundsReward::<Test>::get(CurrentRound::<Test>::get()).store_reward, 0);
			assert_eq!(FileOrders::<Test>::get(&mock_file_id('A')).unwrap().expire_at, 31);
			run_to_block(11);
			assert_ok!(MockData::new(3, 4, 10 * MB, &[]).report_data(3).call(2));
			run_to_block(21);
			assert_ok!(MockData::new(4, 5, 10 * MB, &[]).report_data(3).call(2));
			run_to_block(31);
			assert_ok!(MockData::new(5, 6, 9 * MB, &[])
				.settle_files(&['A'])
				.report_data(3)
				.call(2));
			assert_eq!(
				RoundsReport::<Test>::get(CurrentRound::<Test>::get(), 2).unwrap(),
				NodeStats { power: 9 * MB, used: 0 }
			);
			assert_last_pallet_event!(crate::Event::NodeReported {
				controller: 2,
				machine_id: MACHINES[3].get_machine_id(),
				mine_reward: 0,
				share_store_reward: 0,
				direct_store_reward: 20,
				slash: 0,
			});
			assert_node_info!(2, deposit => default_stash_balance() + 20);
			assert_eq!(RoundsReward::<Test>::get(CurrentRound::<Test>::get()).store_reward, 80);
			assert_eq!(StoreFiles::<Test>::get(&mock_file_id('A')), None);
		})
}

#[test]
fn report_settle_files_do_not_reward_unhealth_node() {
	ExtBuilder::default()
		.files(vec![(mock_file_id('A'), MB, 1200)])
		.reports(vec![
			(
				2,
				MACHINES[3].register_data(),
				MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(3).clone(),
			),
			(
				3,
				MACHINES[0].register_data(),
				MockData::new(0, 3, 100 * MB, &[('A', MB)]).report_data(0).clone(),
			),
		])
		.build()
		.execute_with(|| {
			run_to_block(11);
			assert_ok!(MockData::new(3, 4, 10 * MB, &[]).report_data(3).call(2));
			assert_ok!(MockData::new(3, 4, 10 * MB, &[]).report_data(0).call(3));
			run_to_block(21);
			assert_ok!(MockData::new(4, 5, 10 * MB, &[]).report_data(0).call(3));
			run_to_block(31);
			assert_ok!(MockData::new(5, 6, 10 * MB, &[])
				.settle_files(&['A'])
				.report_data(0)
				.call(3));
			assert_eq!(FileOrders::<Test>::get(&mock_file_id('A')).unwrap().replicas, vec![3]);
            assert_node_info!(3, deposit => default_stash_balance() + 20);
            assert_node_info!(2, deposit => default_stash_balance(), rid => 4, reported_at => 11, power => 10 * MB, used => 0, slash_used => MB);
			assert_ok!(MockData::new(4, 5, 10 * MB, &[]).report_data(3).call(2));
            assert_last_pallet_event!(crate::Event::NodeReported {
				controller: 2,
				machine_id: MACHINES[3].get_machine_id(),
				mine_reward: 0,
				share_store_reward: 0,
				direct_store_reward: 0,
				slash: 110,
			});
            assert_node_info!(2, deposit => default_stash_balance() - 110, rid => 5, reported_at => 31, power => 10 * MB, used => 0, slash_used => 0);
		})
}

#[test]
fn calculate_mine() {
	ExtBuilder::default()
		.mine_factor(Perbill::from_percent(1))
		.build()
		.execute_with(|| {
			run_to_block(11);
			assert_eq!(CurrentRound::<Test>::get(), 2);
			assert_eq!(StoragePotReserved::<Test>::get(), 0);
			assert_eq!(FileStorage::calculate_mine(2), (0, 0, 0));
			RoundsSummary::<Test>::insert(2, SummaryStats { power: 400 * MB2, used: 0 });
			assert_eq!(FileStorage::calculate_mine(2), (4 * MB2, 0, 4 * MB2));
			StoragePotReserved::<Test>::set(MB2);
			assert_eq!(FileStorage::calculate_mine(2), (4 * MB2, 0, 3 * MB2));
			RoundsReward::<Test>::insert(
				1,
				RewardInfo {
					mine_reward: 2 * MB2,
					store_reward: 2 * MB2,
					paid_mine_reward: MB2,
					paid_store_reward: MB2,
				},
			);
			assert_eq!(FileStorage::calculate_mine(2), (4 * MB2, 0, 1 * MB2));
		})
}

#[test]
fn round_end() {
	ExtBuilder::default()
		.mine_factor(Perbill::from_percent(1))
		.build()
		.execute_with(|| {
			run_to_block(11);
			assert_eq!(CurrentRound::<Test>::get(), 2);
			RoundsSummary::<Test>::insert(2, SummaryStats { power: 400 * MB2, used: 0 });
			StoragePotReserved::<Test>::set(MB2);
			RoundsReward::<Test>::insert(
				1,
				RewardInfo {
					mine_reward: 2 * MB2,
					store_reward: 2 * MB2,
					paid_mine_reward: MB2,
					paid_store_reward: MB2,
				},
			);
			let pb = Balances::free_balance(&FileStorage::account_id());
			StoragePotReserved::<Test>::set(MB2);
			run_to_block(21);
			assert_eq!(Balances::free_balance(&FileStorage::account_id()), pb.saturating_add(MB2));
			assert_eq!(
				RoundsReward::<Test>::get(2),
				RewardInfo {
					mine_reward: 4 * MB2,
					store_reward: 0,
					paid_mine_reward: 0,
					paid_store_reward: 0,
				}
			);
			assert_last_pallet_event!(PalletEvent::RoundEnded { round: 2, mine: MB2 })
		})
}

#[test]
fn round_end_clear_prev_prev_round_data() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(CurrentRound::<Test>::get(), 1);
		assert_eq!(NextRoundAt::<Test>::get(), 10);
		RoundsReport::<Test>::insert(1, 2, NodeStats { power: 100 * MB, used: MB });
		RoundsSummary::<Test>::insert(1, SummaryStats { power: 100 * MB2, used: MB2 });
		RoundsReward::<Test>::insert(
			1,
			RewardInfo {
				mine_reward: 2 * MB2,
				store_reward: 2 * MB2,
				paid_mine_reward: 2 * MB2,
				paid_store_reward: 2 * MB2,
			},
		);
		run_to_block(11);
		assert_eq!(CurrentRound::<Test>::get(), 2);
		assert_eq!(NextRoundAt::<Test>::get(), 20);
		assert_eq!(RoundsReport::<Test>::get(1, 2), Some(NodeStats { power: 100 * MB, used: MB }));
		assert_eq!(RoundsSummary::<Test>::get(1), SummaryStats { power: 100 * MB2, used: MB2 });
		assert_eq!(
			RoundsReward::<Test>::get(1),
			RewardInfo {
				mine_reward: 2 * MB2,
				store_reward: 2 * MB2,
				paid_mine_reward: 2 * MB2,
				paid_store_reward: 2 * MB2
			}
		);
		run_to_block(21);
		run_to_block(31);
		assert_eq!(CurrentRound::<Test>::get(), 4);
		assert_eq!(NextRoundAt::<Test>::get(), 40);
		assert_eq!(RoundsSummary::<Test>::get(1), SummaryStats { power: 0, used: 0 });
		assert_eq!(
			RoundsReward::<Test>::get(1),
			RewardInfo {
				mine_reward: 0,
				store_reward: 0,
				paid_store_reward: 0,
				paid_mine_reward: 0,
			}
		);
		assert_eq!(RoundsReport::<Test>::get(1, 2), None);
	})
}

#[test]
fn slash_offline() {
	let report_data = MockData::new(0, 3, 10 * MB, &[('A', MB)]).report_data(3);
	ExtBuilder::default()
		.files(vec![(mock_file_id('A'), MB, 1200)])
		.reports(vec![(2, MACHINES[3].register_data(), report_data.clone())])
		.build()
		.execute_with(|| {
			run_to_block(11);
			assert_ok!(MockData::new(3, 4, 10 * MB, &[]).report_data(3).call(2));
			run_to_block(21);
			let pot_reserved = StoragePotReserved::<Test>::get();
			run_to_block(31);
			assert_ok!(MockData::new(4, 5, 10 * MB, &[])
				.settle_files(&['A'])
				.report_data(3)
				.call(2));
			assert_last_pallet_event!(crate::Event::NodeReported {
				controller: 2,
				machine_id: MACHINES[3].get_machine_id(),
				mine_reward: 0,
				share_store_reward: 0,
				direct_store_reward: 0,
				slash: 110,
			});

			assert_node_info!(2, deposit => default_stash_balance() - 110);
			assert_eq!(StoragePotReserved::<Test>::get(), pot_reserved.saturating_add(110));
			assert_eq!(FileOrders::<Test>::get(&mock_file_id('A')).unwrap().replicas.len(), 0);
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
		let store_file = StoreFiles::<Test>::get(&mock_file_id('A')).unwrap();
		assert_eq!(
			store_file,
			StoreFile {
				reserved: file_fee.saturating_sub(FILE_BASE_PRICE),
				base_fee: FILE_BASE_PRICE,
				file_size: MB,
				added_at: now_at,
			}
		);
		assert_eq!(Balances::free_balance(&1000), u1000 - file_fee);
		assert_eq!(balance_of_storage_pot(), pot.saturating_add(file_fee));

		// Add more fee
		assert_ok!(FileStorage::store(Origin::signed(1000), mock_file_id('A'), MB, 10,));
		let store_file = StoreFiles::<Test>::get(&mock_file_id('A')).unwrap();
		assert_eq!(
			store_file,
			StoreFile {
				reserved: file_fee.saturating_sub(FILE_BASE_PRICE).saturating_add(10),
				base_fee: FILE_BASE_PRICE,
				file_size: MB,
				added_at: now_at
			}
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
			assert_eq!(StoreFiles::<Test>::get(&mock_file_id('A')), None);
			assert_eq!(StoragePotReserved::<Test>::get(), 1100);
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
