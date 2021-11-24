use concordium_cis1::*;
use concordium_std::*;

type TokenId = TokenIdU8;
type TokenCount = u64;

#[derive(Serial)]
enum Keys {
    SimpleMap,
    TokenState,
    TokenCount,
}

#[derive(Serialize, SchemaType)]
struct MintParams {
    owner:       Address,
    token_id:    TokenId,
    token_count: TokenCount,
}

// With GATs (Generic Associated Types), we could avoid the _ parameter and do:
// init<State: HasContractStateHL> .. {
//    let simple_map: State::Map<u8,u8> = state.new_map();
// }

#[init(contract = "new-state", parameter = "MintParams")]
fn init(_ctx: &impl HasInitContext, state: &mut impl HasContractStateHL) -> InitResult<()> {
    let simple_map: StateMap<u8, u8, _> = state.new_map();
    state.insert(Keys::SimpleMap, simple_map);

    let token_state: StateMap<Address, StateMap<TokenId, TokenCount, _>, _> = state.new_map();
    state.insert(Keys::TokenState, token_state);
    state.insert(Keys::TokenCount, 0u64);
    Ok(())
}

#[receive(contract = "new-state", name = "mint", parameter = "MintParams")]
fn receive<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut impl HasContractStateHL,
) -> ReceiveResult<A> {
    let params: MintParams = ctx.parameter_cursor().get()?;

    let mut simple_map: StateMap<u8, u8, _> = state.get(Keys::SimpleMap).unwrap_abort()?;

    // Inserts value in state
    let _ = simple_map.insert(0, 0);
    // Inserts value in state
    let _ = simple_map.insert(1, 2);

    let old_token_count: u64 = state.get(Keys::TokenCount).unwrap_abort()?;
    state.insert(Keys::TokenCount, old_token_count + params.token_count);

    let mut token_state: StateMap<Address, StateMap<TokenId, TokenCount, _>, _> =
        state.get(Keys::TokenState).unwrap_abort()?;

    token_state
        .entry(params.owner)?
        .and_modify(|owner_map| match owner_map.entry(params.token_id) {
            Ok(Entry::Vacant(vac)) => vac.insert(params.token_count),
            Ok(Entry::Occupied(mut occ)) => {
                occ.modify(|current_count| *current_count += params.token_count)
            }
            Err(_) => (),
        })
        .or_insert({
            let mut owner_map = state.new_map();
            let _ = owner_map.insert(params.token_id, params.token_count);
            owner_map
        });

    Ok(A::accept())
}
