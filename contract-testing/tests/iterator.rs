//! This module tests calling a contract which makes use of an iterator.
//! The checks are being performed in the contract itself so if invoking the
//! contract completes successfully then this implies that the tests have done
//! so as well. Note. as per above no checks are being performed in this file
//! wrt. the state etc. after execution etc.

use concordium_smart_contract_testing::*;

const WASM_TEST_FOLDER: &str = "../concordium-base/smart-contracts/testdata/contracts/v1";
const ACC_0: AccountAddress = AccountAddress([0; 32]);

#[test]
fn test_iterator() {
    let mut chain = Chain::new();
    let initial_balance = Amount::from_ccd(1000000);
    chain.create_account(ACC_0, Account::new(initial_balance));

    let res_deploy = chain
        .module_deploy_wasm_v1(ACC_0, format!("{}/iterator.wasm", WASM_TEST_FOLDER))
        .expect("Deploying valid module should work");

    let res_init = chain
        .contract_init(
            ACC_0,
            res_deploy.module_reference,
            ContractName::new_unchecked("init_iterator"),
            OwnedParameter::empty(),
            Amount::zero(),
            Energy::from(10000),
        )
        .expect("Initializing valid contract should work");

    chain
        .contract_update(
            ACC_0,
            Address::Account(ACC_0),
            res_init.contract_address,
            EntrypointName::new_unchecked("iteratetest"),
            OwnedParameter::empty(),
            Amount::zero(),
            Energy::from(10000),
        )
        .expect("Should succeed");
    chain
        .contract_update(
            ACC_0,
            Address::Account(ACC_0),
            res_init.contract_address,
            EntrypointName::new_unchecked("lockingtest"),
            OwnedParameter::empty(),
            Amount::zero(),
            Energy::from(10000),
        )
        .expect("Should succeed.");
}
