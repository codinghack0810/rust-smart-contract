//! CIS2 client is the intermediatory layer between any contract and
//! CIS2 comliant contract.
//!
//! # Description
//! It allows the contract to abstract away the logic of calling the
//! CIS2 contract for the following methods
//! - `supports_cis2` : Calls [`supports`](https://proposals.concordium.software/CIS/cis-0.html#supports)
//! - `operator_of` : Calls [`operatorOf`](https://proposals.concordium.software/CIS/cis-2.html#operatorof)
//! - `balance_of` : Calls [`balanceOf`](https://proposals.concordium.software/CIS/cis-2.html#balanceof)
//! - `transfer` : Calls [`transfer`](https://proposals.concordium.software/CIS/cis-2.html#transfer)
//! - `update_operator` : Calls [`updateOperator`](https://proposals.concordium.software/CIS/cis-2.html#updateoperator)

use crate::*;
use concordium_std::*;

const SUPPORTS_ENTRYPOINT_NAME: EntrypointName = EntrypointName::new_unchecked("supports");
const OPERATOR_OF_ENTRYPOINT_NAME: EntrypointName = EntrypointName::new_unchecked("operatorOf");
const BALANCE_OF_ENTRYPOINT_NAME: EntrypointName = EntrypointName::new_unchecked("balanceOf");
const TRANSFER_ENTRYPOINT_NAME: EntrypointName = EntrypointName::new_unchecked("transfer");
const UPDATE_OPERATOR_ENTRYPOINT_NAME: EntrypointName =
    EntrypointName::new_unchecked("updateOperator");

#[derive(Debug)]
pub struct Cis2ErrorWrapper<T>(Cis2Error<T>);

impl<T> From<Cis2Error<T>> for Cis2ErrorWrapper<T> {
    fn from(e: Cis2Error<T>) -> Self { Cis2ErrorWrapper(e) }
}

impl<T> AsRef<Cis2Error<T>> for Cis2ErrorWrapper<T> {
    fn as_ref(&self) -> &Cis2Error<T> { &self.0 }
}

impl<T: Serial> Serial for Cis2ErrorWrapper<T> {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> { self.0.serial(out) }
}

impl<T: Deserial> Deserial for Cis2ErrorWrapper<T> {
    fn deserial<R: Read>(source: &mut R) -> ParseResult<Self> {
        let tag = source.read_u8()?;
        match tag {
            0 => Ok(Cis2ErrorWrapper(Cis2Error::InvalidTokenId)),
            1 => Ok(Cis2ErrorWrapper(Cis2Error::InsufficientFunds)),
            2 => Ok(Cis2ErrorWrapper(Cis2Error::Unauthorized)),
            3 => {
                let t = T::deserial(source)?;
                Ok(Cis2ErrorWrapper(Cis2Error::Custom(t)))
            }
            _ => bail!(ParseError {}),
        }
    }
}
pub type InvokeContractError<T> = CallContractError<Cis2ErrorWrapper<T>>;

/// Errors which can be returned by the `Cis2Client`.
#[derive(Debug)]
pub enum Cis2ClientError<T> {
    /// When there is an error invoking the CIS2 contract.
    InvokeContractError(InvokeContractError<T>),
    /// When there is an error parsing the result.
    ParseResult,
    /// When the response is invalid. Ex. When the response is empty vector for
    /// a single query.
    InvalidResponse,
}

impl<T: Serial> Serial for Cis2ClientError<T> {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match self {
            Cis2ClientError::InvokeContractError(e) => {
                out.write_u8(3)?;
                match e {
                    CallContractError::AmountTooLarge => out.write_u8(0),
                    CallContractError::MissingAccount => out.write_u8(1),
                    CallContractError::MissingContract => out.write_u8(2),
                    CallContractError::MissingEntrypoint => out.write_u8(3),
                    CallContractError::MessageFailed => out.write_u8(4),
                    CallContractError::LogicReject {
                        reason,
                        return_value,
                    } => {
                        out.write_u8(5)?;
                        reason.serial(out)?;
                        return_value.serial(out)?;
                        Ok(())
                    }
                    CallContractError::Trap => out.write_u8(6),
                }
            }
            Cis2ClientError::ParseResult => out.write_u8(0),
            Cis2ClientError::InvalidResponse => out.write_u8(1),
        }
    }
}

impl<T: Read, R: Deserial> TryFrom<CallContractError<T>> for Cis2ClientError<R> {
    type Error = Cis2ClientError<R>;

    fn try_from(err: CallContractError<T>) -> Result<Cis2ClientError<R>, Cis2ClientError<R>> {
        match err {
            CallContractError::AmountTooLarge => {
                Ok(Cis2ClientError::InvokeContractError(InvokeContractError::AmountTooLarge))
            }
            CallContractError::MissingAccount => {
                Ok(Cis2ClientError::InvokeContractError(InvokeContractError::MissingAccount))
            }
            CallContractError::MissingContract => {
                Ok(Cis2ClientError::InvokeContractError(InvokeContractError::MissingContract))
            }
            CallContractError::MissingEntrypoint => {
                Ok(Cis2ClientError::InvokeContractError(InvokeContractError::MissingEntrypoint))
            }
            CallContractError::MessageFailed => {
                Ok(Cis2ClientError::InvokeContractError(InvokeContractError::MessageFailed))
            }
            CallContractError::LogicReject {
                reason,
                mut return_value,
            } => Ok(Cis2ClientError::InvokeContractError(InvokeContractError::LogicReject {
                reason,
                return_value: {
                    let cis2_error = Cis2ErrorWrapper::<R>::deserial(&mut return_value);
                    match cis2_error {
                        Ok(cis2_error) => cis2_error,
                        Err(_) => bail!(Cis2ClientError::ParseResult),
                    }
                },
            })),
            CallContractError::Trap => {
                Ok(Cis2ClientError::InvokeContractError(InvokeContractError::Trap))
            }
        }
    }
}

impl<T> From<ParseError> for Cis2ClientError<T> {
    fn from(_: ParseError) -> Self { Cis2ClientError::ParseResult }
}

/// `Cis2Client`
/// # Examples
/// ```rust
/// use concordium_cis2::cis2_client::Cis2Client;
/// use concordium_std::ContractAddress;
/// let cis_contract_address = ContractAddress::new(0, 0);
/// Cis2Client::new(cis_contract_address);
/// ```
pub struct Cis2Client {
    contract: ContractAddress,
}

impl Cis2Client {
    pub fn new(contract: ContractAddress) -> Self {
        Self {
            contract,
        }
    }

    /// Calls the `supports` entrypoint of the CIS2 contract to check if the
    /// given contract supports CIS2 standard.
    /// If the contract supports CIS2 standard, it returns
    /// `Ok(SupportResult::Support)`, else it returns
    /// `Ok(SupportResult::NoSupport)`. If the contract supports CIS2
    /// standard by another contract, it returns
    /// `Ok(SupportResult::SupportBy(Vec<ContractAddress>))`. If there is an
    /// error, it returns `Err`. # Examples
    /// ```rust
    /// let cis2_client = Cis2Client::new(cis_contract_address);
    /// let res = cis2_client.supports_cis2(host);

    /// let res = match res {
    ///    Ok(res) => res,
    ///    Err(e) => bail!(),
    /// };
    /// ```
    pub fn supports_cis2<State, E: Deserial>(
        &self,
        host: &impl HasHost<State>,
    ) -> Result<SupportResult, Cis2ClientError<E>> {
        let params = SupportsQueryParams {
            queries: vec![CIS2_STANDARD_IDENTIFIER.to_owned()],
        };
        let mut res: SupportsQueryResponse =
            self.invoke_contract_read_only(host, SUPPORTS_ENTRYPOINT_NAME, &params)?;
        Cis2Client::first(&mut res.results)
    }

    /// Calls the `operatorOf` entrypoint of the CIS2 contract to check if the
    /// given owner is an operator of the given contract. If the owner is an
    /// operator of the given contract, it returns `Ok(true)`,
    /// else it returns `Ok(false)`.
    /// If there is an error, it returns `Err`.
    /// # Examples
    /// ```rust
    /// let cis2_client = Cis2Client::new(cis_contract_address);
    /// let res = cis2_client.operator_of(host, ctx.sender(), Address::Contract(ctx.self_address()));
    /// let res = match res {
    ///     Ok(res) => res,
    ///     Err(e) => bail!(),
    /// };
    pub fn operator_of<State, E: Deserial>(
        &self,
        host: &impl HasHost<State>,
        owner: Address,
        address: Address,
    ) -> Result<bool, Cis2ClientError<E>> {
        let params = &OperatorOfQueryParams {
            queries: vec![OperatorOfQuery {
                owner,
                address,
            }],
        };
        let mut res: OperatorOfQueryResponse =
            self.invoke_contract_read_only(host, OPERATOR_OF_ENTRYPOINT_NAME, params)?;
        Cis2Client::first(&mut res.0)
    }

    /// calls the `balanceOf` entrypoint of the CIS2 contract to get the balance
    /// of the given owner for the given token. If the balance is returned,
    /// it returns `Ok(balance)`, else it returns `Err`.
    /// # Examples
    /// ```rust
    /// let cis2_client = Cis2Client::new(cis_contract_address);
    /// let res = cis2_client.balance_of(host, token_id, Address::Account(owner));
    /// let res: A = match res {
    ///     Ok(res) => res,
    ///     Err(e) => bail!(),
    /// };
    /// ```
    pub fn balance_of<State, T: IsTokenId, A: IsTokenAmount, E: Deserial>(
        &self,
        host: &impl HasHost<State>,
        token_id: T,
        address: Address,
    ) -> Result<A, Cis2ClientError<E>> {
        let params = BalanceOfQueryParams {
            queries: vec![BalanceOfQuery {
                token_id,
                address,
            }],
        };

        let mut res: BalanceOfQueryResponse<A> =
            self.invoke_contract_read_only(host, BALANCE_OF_ENTRYPOINT_NAME, &params)?;
        Cis2Client::first(&mut res.0)
    }

    /// Calls the `transfer` entrypoint of the CIS2 contract to transfer the
    /// given amount of tokens from the given owner to the given receiver.
    /// If the transfer is successful, it returns `Ok(())`, else it returns an
    /// `Err`. # Examples
    /// ```rust
    /// let cis2_client = Cis2Client::new(cis2_contract_address);
    /// let res = cis2_client.transfer(
    ///     host,
    ///     Transfer {
    ///         amount: params.quantity,
    ///         from: Address::Account(params.owner),
    ///         to: Receiver::Account(params.to),
    ///         token_id: params.token_id,
    ///         data: AdditionalData::empty(),
    ///     },
    /// );
    ///
    /// match res {
    ///     Ok(res) => res,
    ///     Err(e) => bail!(MarketplaceError::Cis2ClientError(e)),
    /// };
    pub fn transfer<State, T: IsTokenId, A: IsTokenAmount, E: Deserial>(
        &self,
        host: &mut impl HasHost<State>,
        transfer: Transfer<T, A>,
    ) -> Result<bool, Cis2ClientError<E>> {
        let params = TransferParams(vec![transfer]);
        let (state_modified, _): (bool, Option<()>) =
            self.invoke_contract(host, TRANSFER_ENTRYPOINT_NAME, &params)?;

        Ok(state_modified)
    }

    /// Calls the `updateOperator` of the CIS2 contract.
    /// If the update is successful, it returns `Ok(())`, else it returns an
    /// `Err`. # Examples
    /// ```rust
    /// let client = Cis2Client::new(cis_contract_address);
    /// let res: Result<bool, Cis2ClientError<()>> =
    ///     client.update_operator(&mut host, operator, update);
    ///
    /// assert!(res.is_ok());
    /// ```
    pub fn update_operator<State, E: Deserial>(
        &self,
        host: &mut impl HasHost<State>,
        operator: Address,
        update: OperatorUpdate,
    ) -> Result<bool, Cis2ClientError<E>> {
        let params = UpdateOperator {
            operator,
            update,
        };
        let (state_modified, _): (bool, Option<()>) =
            self.invoke_contract(host, UPDATE_OPERATOR_ENTRYPOINT_NAME, &params)?;

        Ok(state_modified)
    }

    fn invoke_contract_read_only<State, P: Serial, R: Deserial, E: Deserial>(
        &self,
        host: &impl HasHost<State>,
        method: EntrypointName,
        parameter: &P,
    ) -> Result<R, Cis2ClientError<E>> {
        let res =
            host.invoke_contract_read_only(&self.contract, parameter, method, Amount::from_ccd(0));

        let res = match res {
            Ok(val) => val,
            Err(err) => return Err(Cis2ClientError::<E>::try_from(err)?),
        };

        let res = match res {
            // Since the contract should return a response. If it doesn't, it is an error.
            Some(mut res) => R::deserial(&mut res)?,
            None => bail!(Cis2ClientError::InvalidResponse),
        };

        Ok(res)
    }

    fn invoke_contract<State, P: Serial, R: Deserial, E: Deserial>(
        &self,
        host: &mut impl HasHost<State>,
        method: EntrypointName,
        parameter: &P,
    ) -> Result<(bool, Option<R>), Cis2ClientError<E>> {
        let res = host.invoke_contract(&self.contract, parameter, method, Amount::from_ccd(0));

        let res = match res {
            Ok(val) => {
                let o = match val.1 {
                    Some(mut res) => Some(R::deserial(&mut res)?),
                    None => None,
                };
                (val.0, o)
            }
            Err(err) => return Err(Cis2ClientError::<E>::try_from(err)?),
        };

        Ok(res)
    }

    fn first<T, E>(array: &mut Vec<T>) -> Result<T, Cis2ClientError<E>> {
        // If the contract returns a response, but the response is empty, it is an
        // error. Since for a single query the response should be non-empty.
        ensure!(!array.is_empty(), Cis2ClientError::InvalidResponse);

        Ok(array.swap_remove(0))
    }
}

#[cfg(test)]
mod test {
    use crate::cis2_client::*;

    use super::Cis2Client;
    use concordium_std::test_infrastructure::*;

    #[derive(Serial, Deserial, Clone)]
    pub struct TestState;

    const INDEX: u64 = 0;
    const SUBINDEX: u64 = 0;
    type ContractTokenId = TokenIdU8;
    type ContractTokenAmount = TokenAmountU8;

    #[test]
    fn supports_cis2_test_support() {
        let state = TestState {};
        let state_builder = TestStateBuilder::new();
        let mut host = TestHost::new(state, state_builder);
        let cis_contract_address = ContractAddress::new(INDEX, SUBINDEX);
        fn mock_supports(
            parameter: Parameter,
            _a: Amount,
            _a2: &mut Amount,
            _s: &mut TestState,
        ) -> Result<(bool, SupportsQueryResponse), CallContractError<SupportsQueryResponse>>
        {
            // Check that parameters are deserialized correctly.
            let mut cursor = Cursor::new(parameter);
            let params: Result<SupportsQueryParams, ParseError> =
                SupportsQueryParams::deserial(&mut cursor);
            assert!(params.is_ok());
            let params = params.unwrap();
            assert_eq!(
                params.queries[0],
                StandardIdentifierOwned::new_unchecked("CIS-2".to_owned())
            );

            // Return a response with support.
            Ok((false, SupportsQueryResponse {
                results: vec![SupportResult::Support],
            }))
        }

        host.setup_mock_entrypoint(
            cis_contract_address,
            OwnedEntrypointName::new_unchecked("supports".to_string()),
            MockFn::new_v1(mock_supports),
        );

        let client = Cis2Client::new(cis_contract_address);
        let res: Result<SupportResult, Cis2ClientError<()>> = client.supports_cis2(&host);
        assert!(res.is_ok());
        match res.unwrap() {
            SupportResult::NoSupport => fail!(),
            SupportResult::Support => (),
            SupportResult::SupportBy(_) => fail!(),
        }
    }

    #[test]
    fn supports_cis2_test_no_support() {
        let state = TestState {};
        let state_builder = TestStateBuilder::new();
        let mut host = TestHost::new(state, state_builder);
        let cis_contract_address = ContractAddress::new(INDEX, SUBINDEX);
        fn mock_supports(
            _p: Parameter,
            _a: Amount,
            _a2: &mut Amount,
            _s: &mut TestState,
        ) -> Result<(bool, SupportsQueryResponse), CallContractError<SupportsQueryResponse>>
        {
            Ok((false, SupportsQueryResponse {
                results: vec![SupportResult::NoSupport],
            }))
        }

        host.setup_mock_entrypoint(
            cis_contract_address,
            OwnedEntrypointName::new_unchecked("supports".to_string()),
            MockFn::new_v1(mock_supports),
        );

        let client = Cis2Client::new(cis_contract_address);
        let res: Result<SupportResult, Cis2ClientError<()>> = client.supports_cis2(&host);
        assert!(res.is_ok());
        match res.unwrap() {
            SupportResult::NoSupport => (),
            SupportResult::Support => fail!(),
            SupportResult::SupportBy(_) => fail!(),
        }
    }

    #[test]
    fn supports_cis2_test_supported_by_other_contract() {
        let state = TestState {};
        let state_builder = TestStateBuilder::new();
        let mut host = TestHost::new(state, state_builder);
        let cis_contract_address = ContractAddress::new(INDEX, SUBINDEX);
        fn mock_supports(
            _p: Parameter,
            _a: Amount,
            _a2: &mut Amount,
            _s: &mut TestState,
        ) -> Result<(bool, SupportsQueryResponse), CallContractError<SupportsQueryResponse>>
        {
            Ok((false, SupportsQueryResponse {
                results: vec![SupportResult::SupportBy(vec![ContractAddress::new(
                    INDEX,
                    SUBINDEX + 1,
                )])],
            }))
        }

        host.setup_mock_entrypoint(
            cis_contract_address,
            OwnedEntrypointName::new_unchecked("supports".to_string()),
            MockFn::new_v1(mock_supports),
        );

        let client = Cis2Client::new(cis_contract_address);
        let res: Result<SupportResult, Cis2ClientError<()>> = client.supports_cis2(&host);
        match res.unwrap() {
            SupportResult::NoSupport => fail!(),
            SupportResult::Support => fail!(),
            SupportResult::SupportBy(addresses) => {
                assert_eq!(addresses.first(), Some(&ContractAddress::new(INDEX, SUBINDEX + 1)))
            }
        }
    }

    #[test]
    fn operator_of_test() {
        let state = TestState {};
        let state_builder = TestStateBuilder::new();
        let mut host = TestHost::new(state, state_builder);
        let cis_contract_address = ContractAddress::new(INDEX, SUBINDEX);
        let owner = Address::Account(AccountAddress([1; 32]));
        let current_contract_address = Address::Contract(ContractAddress::new(INDEX + 1, SUBINDEX));
        fn mock_operator_of(
            parameter: Parameter,
            _a: Amount,
            _a2: &mut Amount,
            _s: &mut TestState,
        ) -> Result<(bool, OperatorOfQueryResponse), CallContractError<OperatorOfQueryResponse>>
        {
            // Check that parameters are deserialized correctly.
            let mut cursor = Cursor::new(parameter);
            let params: Result<OperatorOfQueryParams, ParseError> =
                OperatorOfQueryParams::deserial(&mut cursor);
            assert!(params.is_ok());
            let params = params.unwrap();
            assert_eq!(
                params.queries[0].address,
                Address::Contract(ContractAddress::new(INDEX + 1, SUBINDEX))
            );
            assert_eq!(params.queries[0].owner, Address::Account(AccountAddress([1; 32])));

            // Return a response with operator true.
            Ok((false, OperatorOfQueryResponse {
                0: vec![true],
            }))
        }

        host.setup_mock_entrypoint(
            cis_contract_address,
            OwnedEntrypointName::new_unchecked("operatorOf".to_string()),
            MockFn::new_v1(mock_operator_of),
        );

        let client = Cis2Client::new(cis_contract_address);
        let res: Result<bool, Cis2ClientError<()>> =
            client.operator_of(&mut host, owner, current_contract_address);

        assert_eq!(res.unwrap(), true);
    }

    #[test]
    fn balance_of_test() {
        let state = TestState {};
        let state_builder = TestStateBuilder::new();
        let mut host = TestHost::new(state, state_builder);
        let cis_contract_address = ContractAddress::new(INDEX, SUBINDEX);
        let owner = Address::Account(AccountAddress([1; 32]));
        fn mock_balance_of(
            parameter: Parameter,
            _a: Amount,
            _a2: &mut Amount,
            _s: &mut TestState,
        ) -> Result<
            (bool, BalanceOfQueryResponse<ContractTokenAmount>),
            CallContractError<BalanceOfQueryResponse<ContractTokenAmount>>,
        > {
            // Check that parameters are deserialized correctly.
            let mut cursor = Cursor::new(parameter);
            let params: Result<BalanceOfQueryParams<ContractTokenId>, ParseError> =
                BalanceOfQueryParams::deserial(&mut cursor);
            assert!(params.is_ok());
            let params = params.unwrap();
            assert_eq!(params.queries[0].token_id, TokenIdU8(1));
            assert_eq!(params.queries[0].address, Address::Account(AccountAddress([1; 32])));

            // Return a balance of 1.
            Ok((false, BalanceOfQueryResponse(vec![1.into()])))
        }

        host.setup_mock_entrypoint(
            cis_contract_address,
            OwnedEntrypointName::new_unchecked("balanceOf".to_string()),
            MockFn::new_v1(mock_balance_of),
        );

        let client = Cis2Client::new(cis_contract_address);
        let res: Result<TokenAmountU8, Cis2ClientError<()>> =
            client.balance_of(&host, TokenIdU8(1), owner);

        assert!(res.is_ok());
        let res: ContractTokenAmount = res.unwrap();
        assert_eq!(res, 1.into());
    }

    #[test]
    fn transfer_test() {
        let state = TestState {};
        let state_builder = TestStateBuilder::new();
        let mut host = TestHost::new(state, state_builder);
        let cis_contract_address = ContractAddress::new(INDEX, SUBINDEX);
        let from = Address::Account(AccountAddress([1; 32]));
        let to_account = AccountAddress([2; 32]);
        let amount: ContractTokenAmount = 1.into();

        fn mock_transfer(
            parameter: Parameter,
            _a: Amount,
            _a2: &mut Amount,
            _s: &mut TestState,
        ) -> Result<(bool, ()), CallContractError<()>> {
            // Check that parameters are deserialized correctly.
            let mut cursor = Cursor::new(parameter);
            let params: Result<TransferParams<ContractTokenId, ContractTokenAmount>, ParseError> =
                TransferParams::deserial(&mut cursor);
            assert!(params.is_ok());
            let params = params.unwrap();
            assert_eq!(params.0[0].token_id, TokenIdU8(1));
            assert_eq!(params.0[0].to.address(), Address::Account(AccountAddress([2; 32])));
            assert_eq!(params.0[0].amount, 1.into());

            // Return a successful transfer.
            Ok((false, ()))
        }

        host.setup_mock_entrypoint(
            cis_contract_address,
            OwnedEntrypointName::new_unchecked("transfer".to_string()),
            MockFn::new_v1(mock_transfer),
        );

        let client = Cis2Client::new(cis_contract_address);
        let res: Result<bool, Cis2ClientError<()>> = client.transfer(&mut host, Transfer {
            amount,
            from,
            to: Receiver::Account(to_account),
            token_id: TokenIdU8(1),
            data: AdditionalData::empty(),
        });

        assert!(res.is_ok());
    }

    #[test]
    fn update_operator_test() {
        let state = TestState {};
        let state_builder = TestStateBuilder::new();
        let mut host = TestHost::new(state, state_builder);
        let cis_contract_address = ContractAddress::new(INDEX, SUBINDEX);
        let operator = Address::Account(AccountAddress([1; 32]));
        let update = OperatorUpdate::Add;

        fn mock_update_operator(
            parameter: Parameter,
            _a: Amount,
            _a2: &mut Amount,
            _s: &mut TestState,
        ) -> Result<(bool, ()), CallContractError<()>> {
            // Check that parameters are deserialized correctly.
            let mut cursor = Cursor::new(parameter);
            let params: Result<UpdateOperator, ParseError> = UpdateOperator::deserial(&mut cursor);
            assert!(params.is_ok());
            let params = params.unwrap();
            assert_eq!(params.operator, Address::Account(AccountAddress([1; 32])));
            match params.update {
                OperatorUpdate::Add => (),
                OperatorUpdate::Remove => fail!(),
            }

            // Return a successful update.
            Ok((false, ()))
        }

        host.setup_mock_entrypoint(
            cis_contract_address,
            OwnedEntrypointName::new_unchecked("updateOperator".to_string()),
            MockFn::new_v1(mock_update_operator),
        );

        let client = Cis2Client::new(cis_contract_address);
        let res: Result<bool, Cis2ClientError<()>> =
            client.update_operator(&mut host, operator, update);

        assert!(res.is_ok());
    }
}
