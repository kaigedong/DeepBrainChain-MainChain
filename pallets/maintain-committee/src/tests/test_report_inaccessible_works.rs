use super::super::mock::*;
use super::super::Error;
use crate::types::{MTOrderStatus, ReportStatus};
use frame_support::{assert_noop, assert_ok};
use once_cell::sync::Lazy;
use std::convert::TryInto;

// 报告机器被租用，但是无法访问
// case1: 只有1委员会预订，同意报告
// case2: 只有1委员会预订，拒绝报告
// case3: 只有1人预订，提交了Hash, 未提交最终结果
// case4: 只有1人预订，未提交Hash, 未提交最终结果

// case5: 有3人预订，都同意报告(最普通的情况)
// case6: 有3人预订，2同意1反对
// case7: 有3人预订，1同意2反对

// case8: 有3人预订，0同意3反对

// case9: 2人预订，都同意
// case10: 2人预订，都反对
// case11: 2人预订，一同意，一反对

const committee: Lazy<sp_core::sr25519::Public> = Lazy::new(|| sr25519::Public::from(Sr25519Keyring::One));
const reporter: Lazy<sp_core::sr25519::Public> = Lazy::new(|| sr25519::Public::from(Sr25519Keyring::Two));
const machine_stash: Lazy<sp_core::sr25519::Public> = Lazy::new(|| sr25519::Public::from(Sr25519Keyring::Ferdie));

const machine_id: Lazy<Vec<u8>> =
    Lazy::new(|| "8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48".as_bytes().to_vec());

// 报告机器被租用，但是无法访问: 只有一个人预订，10分钟后检查结果，两天后结果执行
#[test]
fn report_machine_inaccessible_works1() {
    new_test_with_init_params_ext().execute_with(|| {
        // 记录：ReportInfo, LiveReport, ReporterReport 并支付处理所需的金额
        assert_ok!(MaintainCommittee::report_machine_fault(
            Origin::signed(*reporter),
            crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
        ));

        // 判断调用举报之后的状态
        {
            assert_eq!(
                &MaintainCommittee::live_report(),
                &crate::MTLiveReportList { bookable_report: vec![0], ..Default::default() }
            );
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    machine_id: machine_id.clone(),
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: crate::ReportStatus::Reported,

                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::reporter_report(&*reporter),
                &crate::ReporterReportList { processing_report: vec![0], ..Default::default() }
            );

            // TODO: 检查free_balance
            // reporter=committee，因此需要质押40000，减去租用机器的租金
            // assert_eq!(Balances::free_balance(&reporter), INIT_BALANCE - 40000 * ONE_DBC - 10 * ONE_DBC);
        }

        // 委员会订阅机器故障报告
        assert_ok!(MaintainCommittee::committee_book_report(Origin::signed(*committee), 0));

        // 检查订阅之后的状态
        // do_report_machine_fault:
        // - Writes:
        // LiveReport, ReportInfo, CommitteeOps, CommitteeOrder, committee pay txFee
        {
            assert_eq!(
                &MaintainCommittee::live_report(),
                &crate::MTLiveReportList { bookable_report: vec![0], ..Default::default() }
            );
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![*committee],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::WaitingBook,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&*committee, 0),
                &crate::MTCommitteeOpsDetail {
                    booked_time: 11,
                    order_status: MTOrderStatus::Verifying,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_order(&*committee),
                &crate::MTCommitteeOrderList { booked_report: vec![0], ..Default::default() }
            );

            assert_eq!(Balances::free_balance(&*committee), INIT_BALANCE - 20000 * ONE_DBC - 10 * ONE_DBC);
        }

        // 委员会首先提交Hash: 内容为 订单ID + 验证人自己的随机数 + 机器是否有问题
        // hash(0abcd1) => 0x73124a023f585b4018b9ed3593c7470a
        let offline_committee_hash: [u8; 16] =
            hex::decode("73124a023f585b4018b9ed3593c7470a").unwrap().try_into().unwrap();
        // - Writes:
        // LiveReport, CommitteeOps, CommitteeOrder, ReportInfo
        assert_ok!(MaintainCommittee::committee_submit_verify_hash(
            Origin::signed(*committee),
            0,
            offline_committee_hash.clone()
        ));

        // 检查状态
        {
            assert_eq!(
                &MaintainCommittee::live_report(),
                &crate::MTLiveReportList { bookable_report: vec![0], ..Default::default() }
            );
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![*committee],
                    hashed_committee: vec![*committee],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::WaitingBook,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&*committee, 0),
                &crate::MTCommitteeOpsDetail {
                    booked_time: 11,
                    confirm_hash: offline_committee_hash,
                    hash_time: 11,
                    order_status: MTOrderStatus::WaitingRaw,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_order(&*committee),
                &crate::MTCommitteeOrderList { booked_report: vec![], hashed_report: vec![0], ..Default::default() }
            );
        }

        run_to_block(21);
        // - Writes:
        // ReportInfo, committee_ops,
        assert_ok!(MaintainCommittee::committee_submit_inaccessible_raw(
            Origin::signed(*committee),
            0,
            "abcd".as_bytes().to_vec(),
            true
        ));

        // 检查提交了确认信息后的状态
        {
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![*committee],
                    hashed_committee: vec![*committee],
                    confirmed_committee: vec![*committee],
                    support_committee: vec![*committee],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::SubmittingRaw,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&*committee, 0),
                &crate::MTCommitteeOpsDetail {
                    booked_time: 11,
                    confirm_hash: offline_committee_hash,
                    hash_time: 11,
                    confirm_time: 22,
                    confirm_result: true,
                    order_status: MTOrderStatus::Finished,
                    ..Default::default()
                }
            );
        }

        run_to_block(23);

        // 检查summary的结果
        // summary_a_inaccessible
        // - Writes:
        // ReportInfo, ReportResult, CommitteeOrder, CommitteeOps
        // LiveReport, UnhandledReportResult, ReporterReport,
        {
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![*committee],
                    hashed_committee: vec![*committee],
                    confirmed_committee: vec![*committee],
                    support_committee: vec![*committee],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::CommitteeConfirmed,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::report_result(0),
                &crate::MTReportResultInfo {
                    report_id: 0,
                    reporter: *reporter,
                    reporter_stake: 1000 * ONE_DBC,
                    reward_committee: vec![*committee],
                    machine_id: machine_id.clone(),
                    machine_stash: *machine_stash,
                    slash_time: 22,
                    slash_exec_time: 22 + 2880 * 2,
                    report_result: crate::ReportResultType::ReportSucceed,
                    slash_result: crate::MCSlashResult::Pending,
                    // inconsistent_committee, unruly_committee, machine_stash,
                    // committee_stake
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_order(&*committee),
                &crate::MTCommitteeOrderList { finished_report: vec![0], ..Default::default() }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&*committee, 0),
                &crate::MTCommitteeOpsDetail {
                    booked_time: 11,
                    confirm_hash: offline_committee_hash,
                    hash_time: 11,
                    confirm_time: 22,
                    confirm_result: true,
                    order_status: crate::MTOrderStatus::Finished,

                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::live_report(),
                &crate::MTLiveReportList { finished_report: vec![0], ..Default::default() }
            );
            let unhandled_report_result: Vec<u64> = vec![0];
            assert_eq!(&MaintainCommittee::unhandled_report_result(22 + 2880 * 2), &unhandled_report_result);
            assert_eq!(
                &MaintainCommittee::reporter_report(&*reporter),
                &crate::ReporterReportList { succeed_report: vec![0], ..Default::default() }
            );
        }

        // TODO: 两天后，根据结果进行惩罚
        // run_to_block(32 + 2880 * 2);
        // TODO: 机器在举报成功后会立即被下线
        // 检查online_profile模块的状态
        {
            assert_eq!(
                OnlineProfile::live_machines(),
                online_profile::LiveMachine { offline_machine: vec![machine_id.clone()], ..Default::default() }
            );
            let machine_info = OnlineProfile::machines_info(machine_id.clone());
            assert_eq!(
                machine_info.machine_status,
                online_profile::MachineStatus::ReporterReportOffline(
                    online_profile::OPSlashReason::RentedInaccessible(11),
                    Box::new(online_profile::MachineStatus::Rented),
                    *reporter,
                    vec![*committee],
                )
            );
        }
    })
}

#[test]
fn report_machine_inaccessible_works2() {
    new_test_with_init_params_ext().execute_with(|| {
        // 记录：ReportInfo, LiveReport, ReporterReport 并支付处理所需的金额
        assert_ok!(MaintainCommittee::report_machine_fault(
            Origin::signed(*reporter),
            crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
        ));

        // 委员会订阅机器故障报告
        assert_ok!(MaintainCommittee::committee_book_report(Origin::signed(*committee), 0));

        // 委员会首先提交Hash: 内容为 订单ID + 验证人自己的随机数 + 机器是否有问题
        // hash(0abcd1) => 0x73124a023f585b4018b9ed3593c7470a
        let offline_committee_hash: [u8; 16] =
            hex::decode("98b18d58d8d3bc2f2037cb8310dd6f0e").unwrap().try_into().unwrap();
        // - Writes:
        // LiveReport, CommitteeOps, CommitteeOrder, ReportInfo
        assert_ok!(MaintainCommittee::committee_submit_verify_hash(
            Origin::signed(*committee),
            0,
            offline_committee_hash.clone()
        ));

        run_to_block(21);
        // - Writes:
        // ReportInfo, committee_ops,
        assert_ok!(MaintainCommittee::committee_submit_inaccessible_raw(
            Origin::signed(*committee),
            0,
            "fedcba111".as_bytes().to_vec(),
            false
        ));

        // 检查提交了确认信息后的状态
        {
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![*committee],
                    hashed_committee: vec![*committee],
                    confirmed_committee: vec![*committee],
                    // support_committee: vec![committee],
                    against_committee: vec![*committee],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::SubmittingRaw,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&*committee, 0),
                &crate::MTCommitteeOpsDetail {
                    booked_time: 11,
                    confirm_hash: offline_committee_hash,
                    hash_time: 11,
                    confirm_time: 22,
                    confirm_result: false,
                    order_status: MTOrderStatus::Finished,
                    ..Default::default()
                }
            );
        }

        run_to_block(23);

        // 检查summary的结果
        // summary_a_inaccessible
        // - Writes:
        // ReportInfo, ReportResult, CommitteeOrder, CommitteeOps
        // LiveReport, UnhandledReportResult, ReporterReport,
        {
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![*committee],
                    hashed_committee: vec![*committee],
                    confirmed_committee: vec![*committee],
                    // support_committee: vec![committee],
                    against_committee: vec![*committee],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::CommitteeConfirmed,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::report_result(0),
                &crate::MTReportResultInfo {
                    report_id: 0,
                    reporter: *reporter,
                    reporter_stake: 1000 * ONE_DBC,
                    reward_committee: vec![*committee],
                    machine_id: machine_id.clone(),
                    machine_stash: *machine_stash,
                    slash_time: 22,
                    slash_exec_time: 22 + 2880 * 2,
                    report_result: crate::ReportResultType::ReportRefused,
                    slash_result: crate::MCSlashResult::Pending,
                    // inconsistent_committee, unruly_committee, machine_stash,
                    // committee_stake,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_order(&*committee),
                &crate::MTCommitteeOrderList { finished_report: vec![0], ..Default::default() }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&*committee, 0),
                &crate::MTCommitteeOpsDetail {
                    booked_time: 11,
                    confirm_hash: offline_committee_hash,
                    hash_time: 11,
                    confirm_time: 22,
                    confirm_result: false,
                    order_status: crate::MTOrderStatus::Finished,

                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::live_report(),
                &crate::MTLiveReportList { finished_report: vec![0], ..Default::default() }
            );
            let unhandled_report_result: Vec<u64> = vec![0];
            assert_eq!(&MaintainCommittee::unhandled_report_result(22 + 2880 * 2), &unhandled_report_result);
            assert_eq!(
                &MaintainCommittee::reporter_report(&*reporter),
                &crate::ReporterReportList { failed_report: vec![0], ..Default::default() }
            );
        }

        // TODO: 两天后，根据结果进行惩罚
        // TODO: 机器在举报成功后会立即被下线
    })
}

#[test]
fn report_machine_inaccessible_works3() {
    new_test_with_init_params_ext().execute_with(|| {
        // 记录：ReportInfo, LiveReport, ReporterReport 并支付处理所需的金额
        assert_ok!(MaintainCommittee::report_machine_fault(
            Origin::signed(*reporter),
            crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
        ));

        // 委员会订阅机器故障报告
        assert_ok!(MaintainCommittee::committee_book_report(Origin::signed(*committee), 0));

        // 委员会首先提交Hash: 内容为 订单ID + 验证人自己的随机数 + 机器是否有问题
        // hash(0abcd1) => 0x73124a023f585b4018b9ed3593c7470a
        let offline_committee_hash: [u8; 16] =
            hex::decode("98b18d58d8d3bc2f2037cb8310dd6f0e").unwrap().try_into().unwrap();
        // - Writes:
        // LiveReport, CommitteeOps, CommitteeOrder, ReportInfo
        assert_ok!(MaintainCommittee::committee_submit_verify_hash(
            Origin::signed(*committee),
            0,
            offline_committee_hash.clone()
        ));

        run_to_block(34);

        // 检查summary的结果

        // 检查 report_id: 0

        // summary_a_inaccessible
        // - Writes:
        // ReportInfo, ReportResult, CommitteeOrder, CommitteeOps
        // LiveReport, UnhandledReportResult, ReporterReport,
        {
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![*committee],
                    hashed_committee: vec![*committee],
                    // confirmed_committee: vec![],
                    // support_committee: vec![committee],
                    // against_committee: vec![committee],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::SubmittingRaw,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::report_result(0),
                &crate::MTReportResultInfo {
                    report_id: 0,
                    reporter: *reporter,
                    reporter_stake: 0 * ONE_DBC,
                    unruly_committee: vec![*committee],
                    machine_id: machine_id.clone(),
                    machine_stash: *machine_stash,
                    slash_time: 31,
                    slash_exec_time: 31 + 2880 * 2,
                    report_result: crate::ReportResultType::NoConsensus,
                    slash_result: crate::MCSlashResult::Pending,
                    // inconsistent_committee, reward_committee, machine_stash,
                    // committee_stake,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_order(&*committee),
                &crate::MTCommitteeOrderList { ..Default::default() }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&*committee, 0),
                &crate::MTCommitteeOpsDetail {
                    // booked_time: 11,
                    // confirm_result: false,
                    // order_status: crate::MTOrderStatus::Finished,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::live_report(),
                &crate::MTLiveReportList { bookable_report: vec![1], ..Default::default() }
            );
            let unhandled_report_result: Vec<u64> = vec![0];
            assert_eq!(&MaintainCommittee::unhandled_report_result(31 + 2880 * 2), &unhandled_report_result);
            assert_eq!(
                &MaintainCommittee::reporter_report(&*reporter),
                &crate::ReporterReportList { processing_report: vec![1], failed_report: vec![0], ..Default::default() }
            );
        }

        // 检查自动报告的新订单
        // 判断调用举报之后的状态
        {
            assert_eq!(
                &MaintainCommittee::live_report(),
                &crate::MTLiveReportList { bookable_report: vec![1], ..Default::default() }
            );
            assert_eq!(
                &MaintainCommittee::report_info(1),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    machine_id: machine_id.clone(),
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: crate::ReportStatus::Reported,

                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::reporter_report(&*reporter),
                &crate::ReporterReportList { processing_report: vec![1], failed_report: vec![0], ..Default::default() }
            );

            // TODO: 检查free_balance
            // reporter=committee，因此需要质押40000，减去租用机器的租金
            // assert_eq!(Balances::free_balance(&reporter), INIT_BALANCE - 40000 * ONE_DBC - 10 * ONE_DBC);
        }
    })
}

#[test]
fn report_machine_inaccessible_works4() {
    new_test_with_init_params_ext().execute_with(|| {
        // 记录：ReportInfo, LiveReport, ReporterReport 并支付处理所需的金额
        assert_ok!(MaintainCommittee::report_machine_fault(
            Origin::signed(*reporter),
            crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
        ));

        // 委员会订阅机器故障报告
        assert_ok!(MaintainCommittee::committee_book_report(Origin::signed(*committee), 0));

        run_to_block(34);

        // 检查summary的结果

        // 检查 report_id: 0

        // summary_a_inaccessible
        // - Writes:
        // ReportInfo, ReportResult, CommitteeOrder, CommitteeOps
        // LiveReport, UnhandledReportResult, ReporterReport,
        {
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![*committee],
                    // hashed_committee: vec![],
                    // confirmed_committee: vec![],
                    // support_committee: vec![committee],
                    // against_committee: vec![committee],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::SubmittingRaw,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::report_result(0),
                &crate::MTReportResultInfo {
                    report_id: 0,
                    reporter: *reporter,
                    reporter_stake: 0 * ONE_DBC,
                    unruly_committee: vec![*committee],
                    machine_id: machine_id.clone(),
                    machine_stash: *machine_stash,
                    slash_time: 22,
                    slash_exec_time: 22 + 2880 * 2,
                    report_result: crate::ReportResultType::NoConsensus,
                    slash_result: crate::MCSlashResult::Pending,
                    // inconsistent_committee, reward_committee, machine_stash,
                    // committee_stake,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_order(&*committee),
                &crate::MTCommitteeOrderList { ..Default::default() }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&*committee, 0),
                &crate::MTCommitteeOpsDetail {
                    // booked_time: 11,
                    // confirm_result: false,
                    // order_status: crate::MTOrderStatus::Finished,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::live_report(),
                &crate::MTLiveReportList { bookable_report: vec![1], ..Default::default() }
            );
            let unhandled_report_result: Vec<u64> = vec![0];
            assert_eq!(&MaintainCommittee::unhandled_report_result(22 + 2880 * 2), &unhandled_report_result);
            assert_eq!(
                &MaintainCommittee::reporter_report(&*reporter),
                &crate::ReporterReportList { processing_report: vec![1], failed_report: vec![0], ..Default::default() }
            );
        }

        // TODO: 两天后，根据结果进行惩罚
        // TODO: 机器在举报成功后会立即被下线
    })
}

#[test]
fn report_machine_inaccessible_works5() {
    new_test_with_init_params_ext().execute_with(|| {
        let committee1 = sr25519::Public::from(Sr25519Keyring::One).into();
        let committee2 = sr25519::Public::from(Sr25519Keyring::Two).into();
        let committee3 = sr25519::Public::from(Sr25519Keyring::Ferdie).into();

        // 记录：ReportInfo, LiveReport, ReporterReport 并支付处理所需的金额
        assert_ok!(MaintainCommittee::report_machine_fault(
            Origin::signed(*reporter),
            crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
        ));

        // 委员会订阅机器故障报告
        assert_ok!(MaintainCommittee::committee_book_report(Origin::signed(committee1), 0));
        assert_ok!(MaintainCommittee::committee_book_report(Origin::signed(committee2), 0));
        assert_ok!(MaintainCommittee::committee_book_report(Origin::signed(committee3), 0));

        // 检查订阅之后的状态
        // do_report_machine_fault:
        // - Writes:
        // LiveReport, ReportInfo, CommitteeOps, CommitteeOrder, committee pay txFee
        {
            assert_eq!(
                &MaintainCommittee::live_report(),
                &crate::MTLiveReportList { verifying_report: vec![0], ..Default::default() }
            );
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![committee2, committee3, committee1],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::Verifying,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&committee1, 0),
                &crate::MTCommitteeOpsDetail {
                    booked_time: 11,
                    order_status: MTOrderStatus::Verifying,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_order(&committee1),
                &crate::MTCommitteeOrderList { booked_report: vec![0], ..Default::default() }
            );

            assert_eq!(Balances::free_balance(&committee1), INIT_BALANCE - 20000 * ONE_DBC - 10 * ONE_DBC);
        }

        // 委员会首先提交Hash: 内容为 订单ID + 验证人自己的随机数 + 机器是否有问题
        // hash(0abcd1) => 0x73124a023f585b4018b9ed3593c7470a
        let offline_committee_hash1: [u8; 16] =
            hex::decode("73124a023f585b4018b9ed3593c7470a").unwrap().try_into().unwrap();
        let offline_committee_hash2: [u8; 16] =
            hex::decode("d8accc6d4cee5fae13f058016de7d1e8").unwrap().try_into().unwrap();
        let offline_committee_hash3: [u8; 16] =
            hex::decode("0e4fe3f93cf80c52549cc170d8a32a3c").unwrap().try_into().unwrap();

        // - Writes:
        // LiveReport, CommitteeOps, CommitteeOrder, ReportInfo
        assert_ok!(MaintainCommittee::committee_submit_verify_hash(
            Origin::signed(committee1),
            0,
            offline_committee_hash1.clone()
        ));
        // 不允许再次提交, 不允许其他委员会提交相同hash
        {
            assert_noop!(
                MaintainCommittee::committee_submit_verify_hash(
                    Origin::signed(committee1),
                    0,
                    offline_committee_hash1.clone()
                ),
                Error::<TestRuntime>::NotInBookedList
            );
            assert_noop!(
                MaintainCommittee::committee_submit_verify_hash(
                    Origin::signed(committee2),
                    0,
                    offline_committee_hash1.clone()
                ),
                Error::<TestRuntime>::DuplicateHash
            );
        }

        // 另外两个委员会提交
        {
            assert_ok!(MaintainCommittee::committee_submit_verify_hash(
                Origin::signed(committee2),
                0,
                offline_committee_hash2.clone()
            ));
            assert_ok!(MaintainCommittee::committee_submit_verify_hash(
                Origin::signed(committee3),
                0,
                offline_committee_hash3.clone()
            ));
        }

        // 检查状态
        {
            assert_eq!(
                &MaintainCommittee::live_report(),
                &crate::MTLiveReportList { waiting_raw_report: vec![0], ..Default::default() }
            );
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![committee2, committee3, committee1],
                    hashed_committee: vec![committee2, committee3, committee1],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::SubmittingRaw,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&committee1, 0),
                &crate::MTCommitteeOpsDetail {
                    booked_time: 11,
                    confirm_hash: offline_committee_hash1,
                    hash_time: 11,
                    order_status: MTOrderStatus::WaitingRaw,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_order(&committee1),
                &crate::MTCommitteeOrderList { booked_report: vec![], hashed_report: vec![0], ..Default::default() }
            );
        }

        // run_to_block(21); 直接允许开始提交
        // - Writes:
        // 三个委员会提交 Raw
        // ReportInfo, committee_ops,
        {
            assert_ok!(MaintainCommittee::committee_submit_inaccessible_raw(
                Origin::signed(committee1),
                0,
                "abcd".as_bytes().to_vec(),
                true
            ));
            assert_ok!(MaintainCommittee::committee_submit_inaccessible_raw(
                Origin::signed(committee2),
                0,
                "abcd1".as_bytes().to_vec(),
                true
            ));
            assert_ok!(MaintainCommittee::committee_submit_inaccessible_raw(
                Origin::signed(committee3),
                0,
                "abcd2".as_bytes().to_vec(),
                true
            ));
        }

        // 检查提交了确认信息后的状态
        {
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![committee2, committee3, committee1],
                    hashed_committee: vec![committee2, committee3, committee1],
                    confirmed_committee: vec![committee2, committee3, committee1],
                    support_committee: vec![committee2, committee3, committee1],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::SubmittingRaw,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&committee1, 0),
                &crate::MTCommitteeOpsDetail {
                    booked_time: 11,
                    confirm_hash: offline_committee_hash1,
                    hash_time: 11,
                    confirm_time: 11,
                    confirm_result: true,
                    order_status: MTOrderStatus::Finished,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::live_report(),
                &crate::MTLiveReportList { waiting_raw_report: vec![0], ..Default::default() }
            );
        }

        // run_to_block(23); 应该直接调用检查，在下一个块
        run_to_block(12);

        // 检查summary的结果
        // summary_a_inaccessible
        // - Writes:
        // ReportInfo, ReportResult, CommitteeOrder, CommitteeOps
        // LiveReport, UnhandledReportResult, ReporterReport,
        {
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![committee2, committee3, committee1],
                    hashed_committee: vec![committee2, committee3, committee1],
                    confirmed_committee: vec![committee2, committee3, committee1],
                    support_committee: vec![committee2, committee3, committee1],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::CommitteeConfirmed,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::report_result(0),
                &crate::MTReportResultInfo {
                    report_id: 0,
                    reporter: *reporter,
                    reporter_stake: 1000 * ONE_DBC,
                    reward_committee: vec![committee2, committee3, committee1],
                    machine_id: machine_id.clone(),
                    machine_stash: *machine_stash,
                    slash_time: 11,
                    slash_exec_time: 11 + 2880 * 2,
                    report_result: crate::ReportResultType::ReportSucceed,
                    slash_result: crate::MCSlashResult::Pending,
                    // inconsistent_committee, unruly_committee, machine_stash,
                    // committee_stake
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_order(&committee1),
                &crate::MTCommitteeOrderList { finished_report: vec![0], ..Default::default() }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&committee1, 0),
                &crate::MTCommitteeOpsDetail {
                    booked_time: 11,
                    confirm_hash: offline_committee_hash1,
                    hash_time: 11,
                    confirm_time: 11,
                    confirm_result: true,
                    order_status: crate::MTOrderStatus::Finished,

                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::live_report(),
                &crate::MTLiveReportList { finished_report: vec![0], ..Default::default() }
            );
            let unhandled_report_result: Vec<u64> = vec![0];
            assert_eq!(&MaintainCommittee::unhandled_report_result(11 + 2880 * 2), &unhandled_report_result);
            assert_eq!(
                &MaintainCommittee::reporter_report(&*reporter),
                &crate::ReporterReportList { succeed_report: vec![0], ..Default::default() }
            );
        }

        // TODO: 两天后，根据结果进行惩罚
        // TODO: 机器在举报成功后会立即被下线
    })
}

#[test]
fn report_machine_inaccessible_works8() {
    new_test_with_init_params_ext().execute_with(|| {
        let committee1 = sr25519::Public::from(Sr25519Keyring::One).into();
        let committee2 = sr25519::Public::from(Sr25519Keyring::Two).into();
        let committee3 = sr25519::Public::from(Sr25519Keyring::Ferdie).into();

        // 记录：ReportInfo, LiveReport, ReporterReport 并支付处理所需的金额
        assert_ok!(MaintainCommittee::report_machine_fault(
            Origin::signed(*reporter),
            crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
        ));

        // 委员会订阅机器故障报告
        assert_ok!(MaintainCommittee::committee_book_report(Origin::signed(committee1), 0));
        assert_ok!(MaintainCommittee::committee_book_report(Origin::signed(committee2), 0));
        assert_ok!(MaintainCommittee::committee_book_report(Origin::signed(committee3), 0));

        // 委员会首先提交Hash: 内容为 订单ID + 验证人自己的随机数 + 机器是否有问题
        let offline_committee_hash1: [u8; 16] =
            hex::decode("7deb0809cf63ada45a674b26581fec54").unwrap().try_into().unwrap();
        let offline_committee_hash2: [u8; 16] =
            hex::decode("6520a0a36ec1befdad09cda0520937a9").unwrap().try_into().unwrap();
        let offline_committee_hash3: [u8; 16] =
            hex::decode("85f2d038240b5d0fea5b4979fc7b92c1").unwrap().try_into().unwrap();

        // - Writes:
        // LiveReport, CommitteeOps, CommitteeOrder, ReportInfo
        {
            assert_ok!(MaintainCommittee::committee_submit_verify_hash(
                Origin::signed(committee1),
                0,
                offline_committee_hash1.clone()
            ));
            assert_ok!(MaintainCommittee::committee_submit_verify_hash(
                Origin::signed(committee2),
                0,
                offline_committee_hash2.clone()
            ));
            assert_ok!(MaintainCommittee::committee_submit_verify_hash(
                Origin::signed(committee3),
                0,
                offline_committee_hash3.clone()
            ));
        }

        // run_to_block(21); 直接允许开始提交
        // - Writes:
        // ReportInfo, committee_ops,
        // 三个委员会提交
        {
            assert_ok!(MaintainCommittee::committee_submit_inaccessible_raw(
                Origin::signed(committee1),
                0,
                "abcd".as_bytes().to_vec(),
                false
            ));
            assert_ok!(MaintainCommittee::committee_submit_inaccessible_raw(
                Origin::signed(committee2),
                0,
                "abcd1".as_bytes().to_vec(),
                false
            ));
            assert_ok!(MaintainCommittee::committee_submit_inaccessible_raw(
                Origin::signed(committee3),
                0,
                "abcd2".as_bytes().to_vec(),
                false
            ));
        }

        // 检查提交了确认信息后的状态
        {
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![committee2, committee3, committee1],
                    hashed_committee: vec![committee2, committee3, committee1],
                    confirmed_committee: vec![committee2, committee3, committee1],
                    against_committee: vec![committee2, committee3, committee1],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::SubmittingRaw,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&committee1, 0),
                &crate::MTCommitteeOpsDetail {
                    booked_time: 11,
                    confirm_hash: offline_committee_hash1,
                    hash_time: 11,
                    confirm_time: 11,
                    confirm_result: false,
                    order_status: MTOrderStatus::Finished,
                    ..Default::default()
                }
            );
        }

        // run_to_block(23); 应该直接调用检查，在下一个块
        run_to_block(12);

        // 检查summary的结果
        // summary_a_inaccessible
        // - Writes:
        // ReportInfo, ReportResult, CommitteeOrder, CommitteeOps
        // LiveReport, UnhandledReportResult, ReporterReport,
        {
            assert_eq!(
                &MaintainCommittee::report_info(0),
                &crate::MTReportInfoDetail {
                    reporter: *reporter,
                    report_time: 11,
                    reporter_stake: 1000 * ONE_DBC,
                    first_book_time: 11,
                    machine_id: machine_id.clone(),
                    verifying_committee: None,
                    booked_committee: vec![committee2, committee3, committee1],
                    hashed_committee: vec![committee2, committee3, committee1],
                    confirmed_committee: vec![committee2, committee3, committee1],
                    against_committee: vec![committee2, committee3, committee1],
                    confirm_start: 11 + 10,
                    machine_fault_type: crate::MachineFaultType::RentedInaccessible(machine_id.clone()),
                    report_status: ReportStatus::CommitteeConfirmed,
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::report_result(0),
                &crate::MTReportResultInfo {
                    report_id: 0,
                    reporter: *reporter,
                    reporter_stake: 1000 * ONE_DBC,
                    reward_committee: vec![committee2, committee3, committee1],
                    machine_id: machine_id.clone(),
                    machine_stash: *machine_stash,
                    slash_time: 11,
                    slash_exec_time: 11 + 2880 * 2,
                    report_result: crate::ReportResultType::ReportRefused,
                    slash_result: crate::MCSlashResult::Pending,
                    // inconsistent_committee, unruly_committee, machine_stash,
                    // committee_stake
                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::committee_order(&committee1),
                &crate::MTCommitteeOrderList { finished_report: vec![0], ..Default::default() }
            );
            assert_eq!(
                &MaintainCommittee::committee_ops(&committee1, 0),
                &crate::MTCommitteeOpsDetail {
                    booked_time: 11,
                    confirm_hash: offline_committee_hash1,
                    hash_time: 11,
                    confirm_time: 11,
                    confirm_result: false,
                    order_status: crate::MTOrderStatus::Finished,

                    ..Default::default()
                }
            );
            assert_eq!(
                &MaintainCommittee::live_report(),
                &crate::MTLiveReportList { finished_report: vec![0], ..Default::default() }
            );
            let unhandled_report_result: Vec<u64> = vec![0];
            assert_eq!(&MaintainCommittee::unhandled_report_result(11 + 2880 * 2), &unhandled_report_result);
            assert_eq!(
                &MaintainCommittee::reporter_report(&*reporter),
                &crate::ReporterReportList { failed_report: vec![0], ..Default::default() }
            );
        }

        // TODO: 两天后，根据结果进行惩罚
        // TODO: 机器在举报成功后会立即被下线
    })
}
