mod energy_factory_setup;

use energy_factory_setup::*;
use simple_lock::locked_token::LockedTokenAttributes;

use elrond_wasm_debug::{managed_token_id_wrapped, rust_biguint, DebugApi};

#[test]
fn init_test() {
    let _ = SimpleLockEnergySetup::new(energy_factory::contract_obj, token_unstake::contract_obj);
}

#[test]
fn try_lock() {
    let mut setup =
        SimpleLockEnergySetup::new(energy_factory::contract_obj, token_unstake::contract_obj);
    let first_user = setup.first_user.clone();
    setup
        .b_mock
        .set_esdt_balance(&first_user, b"FAKETOKEN-123456", &rust_biguint!(1_000));

    // wrong token
    setup
        .lock(&first_user, b"FAKETOKEN-123456", 1_000, LOCK_OPTIONS[0])
        .assert_user_error("Invalid payment token");

    // invalid lock option
    setup
        .lock(&first_user, BASE_ASSET_TOKEN_ID, USER_BALANCE, 42)
        .assert_user_error("Invalid lock choice");
}

#[test]
fn lock_ok() {
    let mut setup =
        SimpleLockEnergySetup::new(energy_factory::contract_obj, token_unstake::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    let mut current_epoch = 1;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    setup.b_mock.check_esdt_balance(
        &first_user,
        BASE_ASSET_TOKEN_ID,
        &rust_biguint!(half_balance),
    );

    let first_unlock_epoch = to_start_of_month(current_epoch + LOCK_OPTIONS[0]);
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(half_balance),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: first_unlock_epoch,
        }),
    );

    let mut expected_user_energy =
        rust_biguint!(half_balance) * (first_unlock_epoch - current_epoch);
    let mut actual_user_energy = setup.get_user_energy(&first_user);
    assert_eq!(expected_user_energy, actual_user_energy);

    // check energy after half a year
    let half_year_epochs = EPOCHS_IN_YEAR / 2;
    current_epoch += half_year_epochs;
    setup.b_mock.set_block_epoch(current_epoch);

    expected_user_energy -= rust_biguint!(half_balance) * half_year_epochs;
    actual_user_energy = setup.get_user_energy(&first_user);
    assert_eq!(expected_user_energy, actual_user_energy);

    // lock more tokens
    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    setup
        .b_mock
        .check_esdt_balance(&first_user, BASE_ASSET_TOKEN_ID, &rust_biguint!(0));

    let second_unlock_epoch = to_start_of_month(current_epoch + LOCK_OPTIONS[0]);
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        2,
        &rust_biguint!(half_balance),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: second_unlock_epoch,
        }),
    );

    expected_user_energy += rust_biguint!(half_balance) * (second_unlock_epoch - current_epoch);
    actual_user_energy = setup.get_user_energy(&first_user);
    assert_eq!(expected_user_energy, actual_user_energy);

    // try unlock before deadline
    setup
        .unlock(&first_user, 1, half_balance)
        .assert_user_error("Cannot unlock yet");

    // unlock first tokens
    current_epoch = 1 + LOCK_OPTIONS[0];
    setup.b_mock.set_block_epoch(current_epoch);

    setup.unlock(&first_user, 1, half_balance).assert_ok();
    setup.b_mock.check_esdt_balance(
        &first_user,
        BASE_ASSET_TOKEN_ID,
        &rust_biguint!(half_balance),
    );
}

#[test]
fn unlock_early_test() {
    let mut setup =
        SimpleLockEnergySetup::new(energy_factory::contract_obj, token_unstake::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    let mut current_epoch = 0;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    let penalty_percentage = 4_000u64; // 1 year = 4_000
    let expected_penalty_amount = rust_biguint!(half_balance) * penalty_percentage / 10_000u64;
    let penalty_amount = setup.get_penalty_amount(half_balance, LOCK_OPTIONS[0], 0);
    assert_eq!(penalty_amount, expected_penalty_amount);

    setup.unlock_early(&first_user, 1, half_balance).assert_ok();

    let received_token_amount = rust_biguint!(half_balance) - penalty_amount;
    let expected_balance = received_token_amount + half_balance;
    current_epoch += DEFAULT_UNBOND_EPOCHS;
    setup.b_mock.set_block_epoch(current_epoch);
    setup.claim_unlocked_tokens(&first_user);
    setup
        .b_mock
        .check_esdt_balance(&first_user, BASE_ASSET_TOKEN_ID, &expected_balance);

    let expected_energy = rust_biguint!(0);
    let actual_energy = setup.get_user_energy(&first_user);
    assert_eq!(actual_energy, expected_energy);
}

#[test]
fn multiple_early_unlocks_same_week_test() {
    let mut setup =
        SimpleLockEnergySetup::new(energy_factory::contract_obj, token_unstake::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;
    let sixth_balance = half_balance / 3;

    let mut current_epoch = 0;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    let mut penalty_percentage = 4_000u64; // 1 year = 4_000
    let mut expected_penalty_amount = rust_biguint!(sixth_balance) * penalty_percentage / 10_000u64;
    let mut penalty_amount = setup.get_penalty_amount(sixth_balance, LOCK_OPTIONS[0], 0);
    assert_eq!(penalty_amount, expected_penalty_amount);

    // Unlock early 1/3 of the LockedTokens
    setup
        .unlock_early(&first_user, 1, sixth_balance)
        .assert_ok();

    let received_token_amount = rust_biguint!(sixth_balance) - penalty_amount;

    // After first early unlock of the week, fees are cached into Simple Lock SC
    setup.b_mock.check_nft_balance(
        &setup.sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        1,
        &(expected_penalty_amount / 2u64 + 1u64),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 360,
        }),
    );

    // Unlock early the another 1/3 of the LockedTokens, same week
    setup
        .unlock_early(&first_user, 1, sixth_balance)
        .assert_ok();

    penalty_percentage = 4_000u64; // 1 year = 4_000
    expected_penalty_amount = rust_biguint!(sixth_balance) * penalty_percentage / 10_000u64;
    penalty_amount = setup.get_penalty_amount(sixth_balance, LOCK_OPTIONS[0], 0);
    assert_eq!(penalty_amount, expected_penalty_amount);

    let received_token_amount_2 = rust_biguint!(sixth_balance) - penalty_amount;

    // Unlock early the last 1/3 of the LockedTokens, same week
    setup
        .unlock_early(&first_user, 1, sixth_balance)
        .assert_ok();

    // Claim unbonded tokens
    current_epoch += DEFAULT_UNBOND_EPOCHS;
    setup.b_mock.set_block_epoch(current_epoch);
    setup.claim_unlocked_tokens(&first_user);

    penalty_percentage = 4_000u64; // 1 year = 4_000
    expected_penalty_amount = rust_biguint!(sixth_balance) * penalty_percentage / 10_000u64;
    penalty_amount = setup.get_penalty_amount(sixth_balance, LOCK_OPTIONS[0], 0);
    assert_eq!(penalty_amount, expected_penalty_amount);

    let received_token_amount_3 = rust_biguint!(sixth_balance) - penalty_amount;
    let expected_balance =
        &received_token_amount_3 + &received_token_amount_2 + &received_token_amount + half_balance;
    setup
        .b_mock
        .check_esdt_balance(&first_user, BASE_ASSET_TOKEN_ID, &expected_balance);

    // Energy SC stores the fee until the end of the week
    // Doesn't send it to FeeCollector yet
    setup.b_mock.check_nft_balance(
        &setup.sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        2,
        &(expected_penalty_amount * 3u64 / 2u64 + 1u64),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 390,
        }),
    );
}

#[test]
fn multiple_early_unlocks_multiple_weeks_fee_collector_check_test() {
    let mut setup =
        SimpleLockEnergySetup::new(energy_factory::contract_obj, token_unstake::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;
    let quarter_balance = half_balance / 2;

    let mut current_epoch = 0;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    let mut penalty_percentage = 4_000u64; // 1 year = 4_000
    let expected_penalty_amount = rust_biguint!(quarter_balance) * penalty_percentage / 10_000u64;
    let mut penalty_amount = setup.get_penalty_amount(quarter_balance, LOCK_OPTIONS[0], 0);
    assert_eq!(penalty_amount, expected_penalty_amount);

    // Unlock early half of the LockedTokens
    setup
        .unlock_early(&first_user, 1, quarter_balance)
        .assert_ok();

    let received_token_amount = rust_biguint!(quarter_balance) - penalty_amount;

    setup.b_mock.check_nft_balance(
        &setup.sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        1,
        &(&expected_penalty_amount / 2u64 + 1u64),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 360,
        }),
    );

    current_epoch += EPOCHS_IN_WEEK;
    setup.b_mock.set_block_epoch(current_epoch);

    // Unlock early the other 1/4 of the LockedTokens
    setup
        .unlock_early(&first_user, 1, quarter_balance)
        .assert_ok();

    penalty_percentage = 3_922u64; // (360 epochs - 7 epochs) / 360 * 4_000 = 3_922
    let expected_penalty_amount_2 = rust_biguint!(quarter_balance) * penalty_percentage / 10_000u64;
    penalty_amount = setup.get_penalty_amount(quarter_balance, LOCK_OPTIONS[0] - current_epoch, 0);
    assert_eq!(penalty_amount, expected_penalty_amount_2);

    // Claim unbonded tokens
    current_epoch += DEFAULT_UNBOND_EPOCHS;
    setup.b_mock.set_block_epoch(current_epoch);
    setup.claim_unlocked_tokens(&first_user);

    let received_token_amount_2 = rust_biguint!(quarter_balance) - penalty_amount;
    let expected_balance = &received_token_amount_2 + &received_token_amount + half_balance;
    setup
        .b_mock
        .check_esdt_balance(&first_user, BASE_ASSET_TOKEN_ID, &expected_balance);

    setup.b_mock.check_nft_balance(
        &setup.fees_collector_mock,
        LOCKED_TOKEN_ID,
        1,
        &((&expected_penalty_amount + &expected_penalty_amount_2) / 2u64),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 360,
        }),
    );
}

#[test]
fn reduce_lock_period_test() {
    let mut setup =
        SimpleLockEnergySetup::new(energy_factory::contract_obj, token_unstake::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    let current_epoch = 0;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[1],
        )
        .assert_ok();

    let penalty_percentage = 3_333u64; // (6_000 - 4_000) / (10_000 - 4_000) = 3_333
    let expected_penalty_amount = rust_biguint!(half_balance) * penalty_percentage / 10_000u64;
    let penalty_amount = setup.get_penalty_amount(half_balance, LOCK_OPTIONS[1], LOCK_OPTIONS[0]);
    assert_eq!(penalty_amount, expected_penalty_amount);

    setup
        .reduce_lock_period(&first_user, 1, half_balance, LOCK_OPTIONS[0])
        .assert_ok();

    setup.b_mock.check_esdt_balance(
        &first_user,
        BASE_ASSET_TOKEN_ID,
        &rust_biguint!(half_balance),
    );

    let expected_locked_token_balance = rust_biguint!(half_balance) - &penalty_amount;
    let expected_new_unlock_epoch = EPOCHS_IN_YEAR; // from 2 initial years - 1 year = 1 years
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        2,
        &expected_locked_token_balance,
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: expected_new_unlock_epoch,
        }),
    );

    // Energy SC stores the fee until the end of the week
    setup.b_mock.check_nft_balance(
        &setup.sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        1,
        &(penalty_amount / 2u64 + 1u64),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 720,
        }),
    );

    // at this point, the fee collector should not receive any tokens
    setup.b_mock.check_esdt_balance(
        &setup.fees_collector_mock,
        BASE_ASSET_TOKEN_ID,
        &rust_biguint!(0),
    );

    // check new energy amount
    let expected_energy =
        rust_biguint!(expected_new_unlock_epoch - current_epoch) * expected_locked_token_balance;
    let actual_energy = setup.get_user_energy(&first_user);
    assert_eq!(actual_energy, expected_energy);
}

#[test]
fn extend_locking_period_test() {
    let mut setup =
        SimpleLockEnergySetup::new(energy_factory::contract_obj, token_unstake::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    let current_epoch = 1;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    // extend to 3 years - unsupported option
    setup
        .extend_locking_period(
            &first_user,
            LOCKED_TOKEN_ID,
            1,
            half_balance,
            3 * EPOCHS_IN_YEAR,
        )
        .assert_user_error("Invalid lock choice");

    // extend to 10 years
    setup
        .extend_locking_period(
            &first_user,
            LOCKED_TOKEN_ID,
            1,
            half_balance,
            LOCK_OPTIONS[1],
        )
        .assert_ok();

    let new_unlock_epoch = to_start_of_month(current_epoch + LOCK_OPTIONS[1]);
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        2,
        &rust_biguint!(half_balance),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: new_unlock_epoch,
        }),
    );

    let expected_energy = rust_biguint!(new_unlock_epoch - current_epoch) * half_balance;
    let actual_energy = setup.get_user_energy(&first_user);
    assert_eq!(actual_energy, expected_energy);

    // try "extend" to 1 year
    setup
        .extend_locking_period(
            &first_user,
            LOCKED_TOKEN_ID,
            2,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_user_error("New lock period must be longer than the current one");
}

#[test]
fn test_same_token_nonce() {
    let mut setup =
        SimpleLockEnergySetup::new(energy_factory::contract_obj, token_unstake::contract_obj);
    let first_user = setup.first_user.clone();
    let half_balance = USER_BALANCE / 2;

    let mut current_epoch = 1;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    // lock again after 10 epochs
    current_epoch += 10;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            half_balance,
            LOCK_OPTIONS[0],
        )
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 360,
        }),
    );
}
