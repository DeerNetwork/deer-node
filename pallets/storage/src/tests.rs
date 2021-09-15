use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_ok, traits::Currency};

#[test]
fn set_enclave_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(FileStorage::set_enclave(Origin::root(), mock_register_info2().enclave, 100));

		// should shorten period
		assert_ok!(FileStorage::set_enclave(Origin::root(), mock_register_info1().enclave, 100));

		// should not growth period
		assert_err!(
			FileStorage::set_enclave(Origin::root(), mock_register_info1().enclave, 100),
			Error::<Test>::InvalidEnclaveExpire
		);
	});
}

#[test]
fn round() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(CurrentRound::<Test>::get(), 1);
		assert_eq!(NextRoundAt::<Test>::get(), 10);
		run_to_block(10);
		assert_eq!(CurrentRound::<Test>::get(), 2);
		assert_eq!(NextRoundAt::<Test>::get(), 20);
	});
}

#[test]
fn stash_works() {
	ExtBuilder::default().build().execute_with(|| {
		let stash_balance = default_stash_balance();
		let u1_b = Balances::free_balance(1);
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		assert_eq!(Balances::free_balance(1), u1_b.saturating_sub(stash_balance));
		assert_eq!(balance_of_storage_pot(), stash_balance + 1);
		assert_eq!(Stashs::<Test>::get(2).unwrap().deposit, stash_balance);

		// should recharge when account's stash_balance < T::StashBalance
		let stash_balance_x3 = stash_balance.saturating_mul(3);
		change_stash_balance(stash_balance_x3);
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		assert_eq!(Balances::free_balance(1), u1_b.saturating_sub(stash_balance_x3));
		assert_eq!(Stashs::<Test>::get(2).unwrap().deposit, stash_balance_x3);
		assert_eq!(balance_of_storage_pot(), stash_balance_x3 + 1);

		// should do nothing when account's stash_balance > T::StashBalance
		change_stash_balance(stash_balance);
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		assert_eq!(Balances::free_balance(1), u1_b.saturating_sub(stash_balance_x3));
		assert_eq!(Stashs::<Test>::get(2).unwrap().deposit, stash_balance_x3);
		assert_eq!(balance_of_storage_pot(), stash_balance_x3 + 1);

		// one stasher multiple controller
		assert_ok!(FileStorage::stash(Origin::signed(1), 3));

		// should not stash another controller
		assert_err!(FileStorage::stash(Origin::signed(11), 2), Error::<Test>::InvalidStashPair,);
	})
}

#[test]
fn withdraw_works() {
	ExtBuilder::default().build().execute_with(|| {
		let stash_balance = default_stash_balance();
		Balances::make_free_balance_be(&FileStorage::account_id(), 2_000);

		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		Stashs::<Test>::mutate(2, |maybe_stash_info| {
			if let Some(stash_info) = maybe_stash_info {
				stash_info.deposit = stash_info.deposit.saturating_add(1_000)
			}
		});
		let pot_b = balance_of_storage_pot();
		let u1_b = Balances::free_balance(1);
		assert_ok!(FileStorage::withdraw(Origin::signed(2)));
		assert_eq!(Stashs::<Test>::get(2).unwrap().deposit, stash_balance);
		assert_eq!(Balances::free_balance(1), u1_b.saturating_add(1_000));
		assert_eq!(balance_of_storage_pot(), pot_b.saturating_sub(1_000));

		// should not withdraw when account's deposit < T::StashBalance
		assert_ok!(FileStorage::stash(Origin::signed(11), 12));
		assert_err!(FileStorage::withdraw(Origin::signed(12)), Error::<Test>::NoEnoughToWithdraw,);
	})
}

#[test]
fn register_works() {
	ExtBuilder::default().stash(1, 2).build().execute_with(|| {
		let register = mock_register1();
		let machine_id = register.machine_id.clone();
		assert_eq!(Stashs::<Test>::get(2).unwrap().machine_id, None);
		assert_ok!(call_register(2, register));
		assert_eq!(Stashs::<Test>::get(2).unwrap().machine_id.unwrap(), machine_id.clone());
		let register = Registers::<Test>::get(&machine_id).unwrap();
		assert_eq!(register, mock_register_info1());

		// register again with different register info
		assert_ok!(call_register(2, mock_register2()));
		let register = Registers::<Test>::get(&machine_id).unwrap();
		assert_eq!(register, mock_register_info2());

		// Failed when controller is not stashed
		assert_err!(call_register(3, mock_register1()), Error::<Test>::UnstashNode);

		// Failed when machind_id don't match
		let mut register = mock_register1();
		register.machine_id[0] += 1;
		assert_err!(call_register(2, register), Error::<Test>::MismatchMacheId);

		// Failed when enclave is not inclued
		assert_err!(call_register(2, mock_register3()), Error::<Test>::InvalidEnclave);

		// Failed when relady registered machine
		assert_ok!(FileStorage::stash(Origin::signed(1), 3));
		assert_err!(call_register(3, mock_register1()), Error::<Test>::MachineAlreadyRegistered);
	})
}

#[test]
fn report_works() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, mock_register4())
		.files(vec![(mock_file_id('A'), 100, 2000)])
		.build()
		.execute_with(|| {
			assert_ok!(call_report(2, mock_report4()));
		})
}

#[test]
fn report_works_then_check_storage() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, mock_register1())
		.files(vec![(mock_file_id('A'), 100, 2000)])
		.build()
		.execute_with(|| {
			let now_bn = FileStorage::now_bn();
			let current_round = CurrentRound::<Test>::get();

			assert_ok!(call_report(2, mock_report1()));
			let store_file = StoreFiles::<Test>::get(&mock_file_id('A')).unwrap();
			assert_eq!(
				store_file,
				StoreFile { base_fee: 0, file_size: 100, reserved: 900, added_at: now_bn }
			);
			let file_order = FileOrders::<Test>::get(&mock_file_id('A')).unwrap();
			assert_eq!(
				file_order,
				FileOrder { fee: 100, file_size: 100, expire_at: 31, replicas: vec![2] }
			);
			assert_eq!(StoragePotReserved::<Test>::get(), 1000);
			let round_reward = RoundsReward::<Test>::get(current_round);
			assert_eq!(
				round_reward,
				RewardInfo {
					mine_reward: 0,
					store_reward: 0,
					paid_mine_reward: 0,
					paid_store_reward: 0,
				}
			);
			assert_eq!(
				RoundsReport::<Test>::get(current_round, 2).unwrap(),
				NodeStats { power: 200, used: 100 }
			);
			assert_eq!(
				RoundsSummary::<Test>::get(current_round),
				SummaryStats { power: 200, used: 100 }
			);
			assert_eq!(
				Nodes::<Test>::get(2).unwrap(),
				NodeInfo { rid: 3, reported_at: now_bn, power: 200, used: 100 }
			);
			let stash_info = Stashs::<Test>::get(2).unwrap();
			assert_eq!(stash_info.deposit, default_stash_balance());

			// Failed when report twice in same round
			assert_err!(call_report(2, mock_report1()), Error::<Test>::DuplicateReport);
		})
}

#[test]
fn report_works_when_files_are_miss() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, mock_register1())
		.build()
		.execute_with(|| {
			let now_bn = FileStorage::now_bn();
			assert_ok!(call_report(2, mock_report3()));
			assert_eq!(
				Nodes::<Test>::get(2).unwrap(),
				NodeInfo { rid: 3, reported_at: now_bn, power: 200, used: 0 }
			);
		})
}

#[test]
fn file_order_should_be_removed_if_file_size_is_wrong_and_too_small() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, mock_register1())
		.files(vec![(mock_file_id('A'), 100, 1100)])
		.build()
		.execute_with(|| {
			let current_round = CurrentRound::<Test>::get();

			assert_ok!(call_report(2, mock_report2()));

			assert_eq!(StoragePotReserved::<Test>::get(), 1000);
			let stash_info = Stashs::<Test>::get(2).unwrap();
			assert_eq!(stash_info.deposit, default_stash_balance().saturating_add(12));
			assert_eq!(RoundsReward::<Test>::get(current_round).store_reward, 88);

			assert_eq!(StoreFiles::<Test>::get(&mock_file_id('A')), None);
			assert_eq!(FileOrders::<Test>::get(&mock_file_id('A')), None);
		})
}

#[test]
fn file_order_fee_comes_from_storage_pot_reserved_if_lack() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, mock_register1())
		.files(vec![(mock_file_id('A'), 100, 1100)])
		.build()
		.execute_with(|| {
			change_file_byte_price(default_file_byte_price().saturating_mul(2));
			assert_ok!(call_report(2, mock_report1()));
			assert_eq!(StoragePotReserved::<Test>::get(), 900);
			let store_file = StoreFiles::<Test>::get(&mock_file_id('A')).unwrap();
			assert_eq!(store_file.reserved, 0);
			assert_eq!(store_file.base_fee, 0);
			assert_eq!(FileOrders::<Test>::get(&mock_file_id('A')).unwrap().fee, 200);
		})
}

#[test]
fn file_order_replicas_will_be_replace_if_node_fail_to_report() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, mock_register1())
		.files(vec![(mock_file_id('A'), 100, 1100)])
		.reports(vec![
			(6, mock_register4(), mock_report4()),
			(7, mock_register4(), mock_report4()),
			(8, mock_register4(), mock_report4()),
			(9, mock_register4(), mock_report4()),
		])
		.build()
		.execute_with(|| {
			run_to_block(11);
			assert_ok!(call_report(9, mock_report6()));
			run_to_block(21);
			assert_ok!(call_report(2, mock_report1()));
			let file_order = FileOrders::<Test>::get(&mock_file_id('A')).unwrap();
			assert_eq!(file_order.replicas, vec![9, 2]);
			assert_eq!(
				RoundsSummary::<Test>::get(CurrentRound::<Test>::get()),
				SummaryStats { power: 200, used: 100 }
			);
		})
}

#[test]
fn report_del_files() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, mock_register1())
		.files(vec![(mock_file_id('A'), 100, 1100)])
		.build()
		.execute_with(|| {
			assert_ok!(call_register(2, mock_register1()));
			assert_ok!(call_report(2, mock_report1()));
			let file_order = FileOrders::<Test>::get(&mock_file_id('A')).unwrap();
			assert_eq!(file_order.replicas, vec![2]);
			run_to_block(11);
			assert_ok!(call_report(2, mock_report5()));
			let file_order = FileOrders::<Test>::get(&mock_file_id('A')).unwrap();
			assert_eq!(file_order.replicas.len(), 0);
			assert_eq!(
				RoundsReport::<Test>::get(CurrentRound::<Test>::get(), 2).unwrap(),
				NodeStats { power: 100, used: 0 }
			);
		})
}

#[test]
fn report_settle_files() {
	ExtBuilder::default()
		.files(vec![(mock_file_id('A'), 100, 1100)])
		.reports(vec![(9, mock_register4(), mock_report4())])
		.build()
		.execute_with(|| {
			let stash_balance = default_stash_balance();
			assert_eq!(Stashs::<Test>::get(9).unwrap().deposit, stash_balance);
			assert_eq!(RoundsReward::<Test>::get(CurrentRound::<Test>::get()).store_reward, 0);
			let file_order = FileOrders::<Test>::get(&mock_file_id('A')).unwrap();
			assert_eq!(file_order.expire_at, 31);
			run_to_block(11);
			assert_ok!(call_report(9, mock_report6()));
			run_to_block(21);
			assert_ok!(call_report(9, mock_report7()));
			run_to_block(32);
			assert_ok!(call_report(9, mock_report8()));
			assert_eq!(Stashs::<Test>::get(9).unwrap().deposit, stash_balance.saturating_add(12));
			assert_eq!(RoundsReward::<Test>::get(CurrentRound::<Test>::get()).store_reward, 88);
			assert_eq!(StoreFiles::<Test>::get(&mock_file_id('A')).is_none(), true);
		})
}

#[test]
fn report_settle_files_do_not_reward_unhealth_node() {
	ExtBuilder::default()
		.files(vec![(mock_file_id('A'), 100, 1200)])
		.reports(vec![(9, mock_register4(), mock_report4())])
		.build()
		.execute_with(|| {
			let stash_balance = default_stash_balance();
			run_to_block(11);
			assert_ok!(call_report(9, mock_report6()));
			run_to_block(21);
			run_to_block(32);
			assert_ok!(call_report(9, mock_report9()));
			assert_eq!(Stashs::<Test>::get(9).unwrap().deposit, stash_balance.saturating_sub(100)); // slash
			assert_eq!(RoundsReward::<Test>::get(CurrentRound::<Test>::get()).store_reward, 100);
			assert_eq!(FileOrders::<Test>::get(&mock_file_id('A')).unwrap().replicas.len(), 0);
		})
}

#[test]
fn round_reward() {
	ExtBuilder::default()
		.files(vec![(mock_file_id('A'), 100, 1100)])
		.reports(vec![(9, mock_register4(), mock_report4())])
		.build()
		.execute_with(|| {
			let stash_balance = default_stash_balance();
			run_to_block(11);
			assert_ok!(call_report(9, mock_report6()));
			run_to_block(21);
			assert_ok!(call_report(9, mock_report7()));
			RoundsReward::<Test>::mutate(CurrentRound::<Test>::get(), |reward| {
				reward.store_reward = 100;
				reward.mine_reward = 100;
			});
			run_to_block(32);
			assert_ok!(call_report(9, mock_report8()));
			assert_eq!(Stashs::<Test>::get(9).unwrap().deposit, stash_balance.saturating_add(212));
			assert_eq!(RoundsReward::<Test>::get(CurrentRound::<Test>::get()).store_reward, 88);
			assert_eq!(StoreFiles::<Test>::get(&mock_file_id('A')).is_none(), true);
		})
}

#[test]
fn slash_offline() {
	ExtBuilder::default()
		.files(vec![(mock_file_id('A'), 100, 1100)])
		.reports(vec![(9, mock_register4(), mock_report4())])
		.build()
		.execute_with(|| {
			let stash_balance = default_stash_balance();
			run_to_block(11);
			assert_ok!(call_report(9, mock_report6()));
			run_to_block(21);
			let pot_reserved = StoragePotReserved::<Test>::get();
			run_to_block(32);
			assert_ok!(call_report(9, mock_report7()));
			assert_eq!(Stashs::<Test>::get(9).unwrap().deposit, stash_balance.saturating_sub(100));
			assert_eq!(StoragePotReserved::<Test>::get(), pot_reserved.saturating_add(100));
		})
}

#[test]
fn report_failed_with_legal_input() {
	ExtBuilder::default().build().execute_with(|| {
		// Failed when controller is not stashed
		assert_err!(call_report(2, mock_report1()), Error::<Test>::UnstashNode);

		// Failed when controller is not registered
		assert_ok!(FileStorage::stash(Origin::signed(1), 2));
		assert_err!(call_report(2, mock_report1()), Error::<Test>::UnregisterNode);

		assert_ok!(call_register(2, mock_register1()));

		// Failed when add_files or del_files is tampered
		let mut report = mock_report1();
		report.add_files[0].1 = report.add_files[0].1 + 1;
		assert_err!(call_report(2, report), Error::<Test>::InvalidVerifyP256Sig);

		// Failed when enclave is outdated
		run_to_block(1001);
		assert_err!(call_report(2, mock_report1()), Error::<Test>::InvalidEnclave);
	})
}

#[test]
fn report_failed_when_rid_is_not_continuous() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, mock_register1())
		.files(vec![(mock_file_id('A'), 100, 1100)])
		.build()
		.execute_with(|| {
			assert_ok!(call_register(2, mock_register1()));
			assert_ok!(call_report(2, mock_report1()));
			run_to_block(11);
			assert_err!(call_report(2, mock_report1()), Error::<Test>::InvalidVerifyP256Sig);
			Nodes::<Test>::mutate(2, |maybe_node_info| {
				if let Some(node_info) = maybe_node_info {
					node_info.rid = 0;
				}
			});
			assert_ok!(call_report(2, mock_report1()));
		})
}

#[test]
fn store_works() {
	ExtBuilder::default().build().execute_with(|| {
		let pot_b = balance_of_storage_pot();
		let u1000_b = Balances::free_balance(&1000);
		let file_fee = 1100;
		assert_eq!(FileStorage::store_file_fee(2000), file_fee);
		assert_ok!(FileStorage::store(Origin::signed(1000), mock_file_id('A'), 100, file_fee,));
		let now_bn = FileStorage::now_bn();
		let store_file = StoreFiles::<Test>::get(&mock_file_id('A')).unwrap();
		assert_eq!(
			store_file,
			StoreFile {
				reserved: file_fee.saturating_sub(FILE_BASE_PRICE),
				base_fee: FILE_BASE_PRICE,
				file_size: 100,
				added_at: now_bn,
			}
		);
		assert_eq!(Balances::free_balance(&1000), u1000_b - file_fee);
		assert_eq!(balance_of_storage_pot(), pot_b.saturating_add(file_fee));

		// Add more fee
		assert_ok!(FileStorage::store(Origin::signed(1000), mock_file_id('A'), 1000, 10,));
		let store_file = StoreFiles::<Test>::get(&mock_file_id('A')).unwrap();
		assert_eq!(
			store_file,
			StoreFile {
				reserved: file_fee.saturating_sub(FILE_BASE_PRICE).saturating_add(10),
				base_fee: FILE_BASE_PRICE,
				file_size: 100,
				added_at: now_bn
			}
		);

		// Failed when fee is not enough
		assert_err!(
			FileStorage::store(
				Origin::signed(100),
				mock_file_id('B'),
				100,
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
			run_to_block(32);
			assert_eq!(StoragePotReserved::<Test>::get(), 0);
			assert_ok!(FileStorage::force_delete(Origin::root(), mock_file_id('A')));
			assert!(StoreFiles::<Test>::get(&mock_file_id('A')).is_none());
			assert_eq!(StoragePotReserved::<Test>::get(), 1100);
		})
}

#[test]
fn unpaid_reward_to_treasury() {}
