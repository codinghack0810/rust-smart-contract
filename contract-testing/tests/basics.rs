//! This module contains tests for the basic functionality of the testing
//! library.
use concordium_smart_contract_testing::*;

const ACC_0: AccountAddress = AccountAddress([0; 32]);
const ACC_1: AccountAddress = AccountAddress([1; 32]);

#[test]
fn deploying_valid_module_works() {
    let mut chain = Chain::new();
    let initial_balance = Amount::from_ccd(10000);
    chain.create_account(Account::new(ACC_0, initial_balance));

    let res = chain
        .module_deploy_v1(
            Signer::with_one_key(),
            ACC_0,
            Chain::module_load_v1(
                "../../concordium-rust-smart-contracts/examples/icecream/a.wasm.v1",
            )
            .expect("module should exist"),
        )
        .expect("Deploying valid module should work.");

    assert!(chain.get_module(res.module_reference).is_some());
    assert_eq!(
        chain.account_balance_available(ACC_0),
        Some(initial_balance - res.transaction_fee)
    );
}

#[test]
fn initializing_valid_contract_works() {
    let mut chain = Chain::new();
    let initial_balance = Amount::from_ccd(10000);
    chain.create_account(Account::new(ACC_0, initial_balance));

    let res_deploy = chain
        .module_deploy_v1(
            Signer::with_one_key(),
            ACC_0,
            Chain::module_load_v1(
                "../../concordium-rust-smart-contracts/examples/icecream/a.wasm.v1",
            )
            .expect("module should exist"),
        )
        .expect("Deploying valid module should work");

    let res_init = chain
        .contract_init(
            Signer::with_one_key(),
            ACC_0,
            Energy::from(10000),
            InitContractPayload {
                amount:    Amount::zero(),
                mod_ref:   res_deploy.module_reference,
                init_name: OwnedContractName::new_unchecked("init_weather".into()),
                param:     OwnedParameter::try_from(vec![0u8]).expect("Parameter has valid size."),
            },
        )
        .expect("Initializing valid contract should work");
    assert_eq!(
        chain.account_balance_available(ACC_0),
        Some(initial_balance - res_deploy.transaction_fee - res_init.transaction_fee)
    );
    assert!(chain.get_contract(ContractAddress::new(0, 0)).is_some());
}

#[test]
fn initializing_with_invalid_parameter_fails() {
    let mut chain = Chain::new();
    let initial_balance = Amount::from_ccd(10000);
    chain.create_account(Account::new(ACC_0, initial_balance));

    let res_deploy = chain
        .module_deploy_v1(
            Signer::with_one_key(),
            ACC_0,
            Chain::module_load_v1(
                "../../concordium-rust-smart-contracts/examples/icecream/a.wasm.v1",
            )
            .expect("module should exist"),
        )
        .expect("Deploying valid module should work");

    let res_init =
        chain
            .contract_init(
                Signer::with_one_key(),
                ACC_0,
                Energy::from(10000),
                InitContractPayload {
                    amount:    Amount::zero(),
                    mod_ref:   res_deploy.module_reference,
                    init_name: OwnedContractName::new_unchecked("init_weather".into()),
                    param:     OwnedParameter::try_from(vec![99u8])
                        .expect("Parameter has valid size."), // Invalid param
                },
            )
            .expect_err("Initializing with invalid params should fail");

    let transaction_fee = res_init.transaction_fee;
    match res_init.kind {
        // Failed in the right way and account is still charged.
        ContractInitErrorKind::ExecutionError {
            error: InitExecutionError::Reject { .. },
        } => assert_eq!(
            chain.account_balance_available(ACC_0),
            Some(initial_balance - res_deploy.transaction_fee - transaction_fee)
        ),
        _ => panic!("Expected valid chain error."),
    };
}

#[test]
fn updating_valid_contract_works() {
    let mut chain = Chain::new();
    let initial_balance = Amount::from_ccd(10000);
    chain.create_account(Account::new(ACC_0, initial_balance));

    let res_deploy = chain
        .module_deploy_v1(
            Signer::with_one_key(),
            ACC_0,
            Chain::module_load_v1(
                "../../concordium-rust-smart-contracts/examples/icecream/a.wasm.v1",
            )
            .expect("module should exist"),
        )
        .expect("Deploying valid module should work");

    let res_init =
        chain
            .contract_init(
                Signer::with_one_key(),
                ACC_0,
                Energy::from(10000),
                InitContractPayload {
                    amount:    Amount::zero(),
                    mod_ref:   res_deploy.module_reference,
                    init_name: OwnedContractName::new_unchecked("init_weather".into()),
                    param:     OwnedParameter::try_from(vec![0u8])
                        .expect("Parameter has valid size."), // Starts as 0
                },
            )
            .expect("Initializing valid contract should work");

    let res_update = chain
        .contract_update(
            Signer::with_one_key(),
            ACC_0,
            Address::Account(ACC_0),
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::zero(),
                address:      res_init.contract_address,
                receive_name: OwnedReceiveName::new_unchecked("weather.set".into()),
                message:      OwnedParameter::try_from(vec![1u8])
                    .expect("Parameter has valid size."), // Updated to 1
            },
        )
        .expect("Updating valid contract should work");

    let res_invoke_get = chain
        .contract_invoke(
            ACC_0,
            Address::Contract(res_init.contract_address), // Invoke with a contract as sender.
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::zero(),
                address:      res_init.contract_address,
                receive_name: OwnedReceiveName::new_unchecked("weather.get".into()),
                message:      OwnedParameter::empty(),
            },
        )
        .expect("Invoking get should work");

    // This also asserts that the account wasn't charged for the invoke.
    assert_eq!(
        chain.account_balance_available(ACC_0),
        Some(
            initial_balance
                - res_deploy.transaction_fee
                - res_init.transaction_fee
                - res_update.transaction_fee
        )
    );
    assert!(chain.get_contract(res_init.contract_address).is_some());
    assert!(res_update.state_changed);
    // Assert that the updated state is persisted.
    assert_eq!(res_invoke_get.return_value, [1u8]);
}

/// Test that updates and invocations where the sender is missing fail.
#[test]
fn updating_and_invoking_with_missing_sender_fails() {
    let mut chain = Chain::new();
    let initial_balance = Amount::from_ccd(10000);
    chain.create_account(Account::new(ACC_0, initial_balance));

    let missing_account = Address::Account(ACC_1);
    let missing_contract = Address::Contract(ContractAddress::new(100, 0));

    let res_deploy = chain
        .module_deploy_v1(
            Signer::with_one_key(),
            ACC_0,
            Chain::module_load_v1(
                "../../concordium-rust-smart-contracts/examples/icecream/a.wasm.v1",
            )
            .expect("module should exist"),
        )
        .expect("Deploying valid module should work");

    let res_init =
        chain
            .contract_init(
                Signer::with_one_key(),
                ACC_0,
                Energy::from(10000),
                InitContractPayload {
                    amount:    Amount::zero(),
                    mod_ref:   res_deploy.module_reference,
                    init_name: OwnedContractName::new_unchecked("init_weather".into()),
                    param:     OwnedParameter::try_from(vec![0u8])
                        .expect("Parameter has valid size."), // Starts as 0
                },
            )
            .expect("Initializing valid contract should work");

    let res_update_acc = chain
        .contract_update(
            Signer::with_one_key(),
            ACC_0,
            missing_account,
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::zero(),
                address:      res_init.contract_address,
                receive_name: OwnedReceiveName::new_unchecked("weather.get".into()),
                message:      OwnedParameter::empty(),
            },
        )
        .expect_err("should fail");

    let res_invoke_acc = chain
        .contract_invoke(
            ACC_0,
            missing_account,
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::zero(),
                address:      res_init.contract_address,
                receive_name: OwnedReceiveName::new_unchecked("weather.get".into()),
                message:      OwnedParameter::empty(),
            },
        )
        .expect_err("should fail");

    let res_update_contr = chain
        .contract_update(
            Signer::with_one_key(),
            ACC_0,
            missing_contract,
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::zero(),
                address:      res_init.contract_address,
                receive_name: OwnedReceiveName::new_unchecked("weather.get".into()),
                message:      OwnedParameter::empty(),
            },
        )
        .expect_err("should fail");

    let res_invoke_contr = chain
        .contract_invoke(
            ACC_0,
            missing_contract,
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::zero(),
                address:      res_init.contract_address,
                receive_name: OwnedReceiveName::new_unchecked("weather.get".into()),
                message:      OwnedParameter::empty(),
            },
        )
        .expect_err("should fail");

    assert!(matches!(
            res_update_acc.kind,
            ContractInvokeErrorKind::SenderDoesNotExist(addr) if addr == missing_account));
    assert!(matches!(
            res_invoke_acc.kind,
            ContractInvokeErrorKind::SenderDoesNotExist(addr) if addr == missing_account));
    assert!(matches!(
            res_update_contr.kind,
            ContractInvokeErrorKind::SenderDoesNotExist(addr) if addr == missing_contract));
    assert!(matches!(
            res_invoke_contr.kind,
            ContractInvokeErrorKind::SenderDoesNotExist(addr) if addr == missing_contract));
}

#[test]
fn init_with_less_energy_than_module_lookup() {
    let mut chain = Chain::new();
    let initial_balance = Amount::from_ccd(1000000);
    chain.create_account(Account::new(ACC_0, initial_balance));

    let res_deploy = chain
        .module_deploy_v1(
            Signer::with_one_key(),
            ACC_0,
            Chain::module_load_v1("../../concordium-rust-smart-contracts/examples/fib/a.wasm.v1")
                .expect("module should exist"),
        )
        .expect("Deploying valid module should work");

    let reserved_energy = Energy::from(10);

    let res_init = chain.contract_init(
        Signer::with_one_key(),
        ACC_0,
        reserved_energy,
        InitContractPayload {
            amount:  Amount::zero(),
            mod_ref: res_deploy.module_reference,

            init_name: OwnedContractName::new_unchecked("init_fib".into()),
            param:     OwnedParameter::empty(),
        },
    );
    match res_init {
        Err(ContractInitError {
            kind: ContractInitErrorKind::OutOfEnergy,
            ..
        }) => (),
        _ => panic!("Expected to fail with out of energy."),
    }
}

#[test]
fn update_with_fib_reentry_works() {
    let mut chain = Chain::new();
    let initial_balance = Amount::from_ccd(1000000);
    chain.create_account(Account::new(ACC_0, initial_balance));

    let res_deploy = chain
        .module_deploy_v1(
            Signer::with_one_key(),
            ACC_0,
            Chain::module_load_v1("../../concordium-rust-smart-contracts/examples/fib/a.wasm.v1")
                .expect("module should exist"),
        )
        .expect("Deploying valid module should work");

    let res_init = chain
        .contract_init(
            Signer::with_one_key(),
            ACC_0,
            Energy::from(10000),
            InitContractPayload {
                amount:    Amount::zero(),
                mod_ref:   res_deploy.module_reference,
                init_name: OwnedContractName::new_unchecked("init_fib".into()),
                param:     OwnedParameter::empty(),
            },
        )
        .expect("Initializing valid contract should work");

    let res_update = chain
        .contract_update(
            Signer::with_one_key(),
            ACC_0,
            Address::Account(ACC_0),
            Energy::from(100000),
            UpdateContractPayload {
                amount:       Amount::zero(),
                address:      res_init.contract_address,
                receive_name: OwnedReceiveName::new_unchecked("fib.receive".into()),
                message:      OwnedParameter::from_serial(&6u64).expect("Parameter has valid size"),
            },
        )
        .expect("Updating valid contract should work");

    let res_view = chain
        .contract_invoke(
            ACC_0,
            Address::Account(ACC_0),
            Energy::from(10000),
            UpdateContractPayload {
                amount:       Amount::zero(),
                address:      res_init.contract_address,
                receive_name: OwnedReceiveName::new_unchecked("fib.view".into()),
                message:      OwnedParameter::empty(),
            },
        )
        .expect("Invoking get should work");

    // This also asserts that the account wasn't charged for the invoke.
    assert_eq!(
        chain.account_balance_available(ACC_0),
        Some(
            initial_balance
                - res_deploy.transaction_fee
                - res_init.transaction_fee
                - res_update.transaction_fee
        )
    );
    assert!(chain.get_contract(res_init.contract_address).is_some());
    assert!(res_update.state_changed);
    let expected_res = u64::to_le_bytes(13);
    assert_eq!(res_update.return_value, expected_res);
    // Assert that the updated state is persisted.
    assert_eq!(res_view.return_value, expected_res);
}
