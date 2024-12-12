use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

// ===== STATE =====

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Invoice {
    pub id: u64,
    pub issuer: Addr,
    pub recipient: Addr,
    pub amount: Uint128,
    pub description: String,
    pub due_date: u64,
    pub is_paid: bool,
}

pub const NEXT_INVOICE_ID: Item<u64> = Item::new("next_invoice_id");
pub const INVOICES: Map<u64, Invoice> = Map::new("invoices");
pub const USER_INVOICES: Map<Addr, Vec<u64>> = Map::new("user_invoices");

// ===== MESSAGES =====

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ExecuteMsg {
    CreateInvoice {
        recipient: String,
        amount: Uint128,
        description: String,
        due_date: u64,
    },
    PayInvoice { invoice_id: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum QueryMsg {
    GetInvoice { invoice_id: u64 },
    GetUserInvoices { user: String },
}

// ===== INSTANTIATE =====

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    NEXT_INVOICE_ID.save(deps.storage, &1)?; // Initialize the invoice ID counter
    Ok(Response::new().add_attribute("action", "instantiate"))
}

// ===== EXECUTE =====

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::CreateInvoice {
            recipient,
            amount,
            description,
            due_date,
        } => execute_create_invoice(deps, info, recipient, amount, description, due_date),
        ExecuteMsg::PayInvoice { invoice_id } => execute_pay_invoice(deps, info, invoice_id),
    }
}

fn execute_create_invoice(
    deps: DepsMut,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
    description: String,
    due_date: u64,
) -> StdResult<Response> {
    let recipient_addr = deps.api.addr_validate(&recipient)?;
    if info.sender == recipient_addr {
        return Err(StdError::generic_err("Cannot create invoice for yourself"));
    }
    if amount.is_zero() {
        return Err(StdError::generic_err("Amount must be greater than zero"));
    }

    let id = NEXT_INVOICE_ID.load(deps.storage)?;
    NEXT_INVOICE_ID.save(deps.storage, &(id + 1))?;

    let invoice = Invoice {
        id,
        issuer: info.sender.clone(),
        recipient: recipient_addr.clone(),
        amount,
        description,
        due_date,
        is_paid: false,
    };

    INVOICES.save(deps.storage, id, &invoice)?;

    let mut user_invoices = USER_INVOICES.may_load(deps.storage, info.sender.clone())?.unwrap_or_default();
    user_invoices.push(id);
    USER_INVOICES.save(deps.storage, info.sender.clone(), &user_invoices)?;

    Ok(Response::new()
        .add_attribute("action", "create_invoice")
        .add_attribute("issuer", info.sender.to_string())
        .add_attribute("recipient", recipient)
        .add_attribute("invoice_id", id.to_string()))
}

fn execute_pay_invoice(deps: DepsMut, info: MessageInfo, invoice_id: u64) -> StdResult<Response> {
    let mut invoice = INVOICES.load(deps.storage, invoice_id)?;
    if invoice.recipient != info.sender {
        return Err(StdError::generic_err("Only the recipient can pay this invoice"));
    }
    if invoice.is_paid {
        return Err(StdError::generic_err("Invoice is already paid"));
    }
    if info.funds.len() != 1 || info.funds[0].amount != invoice.amount {
        return Err(StdError::generic_err("Incorrect payment amount"));
    }

    invoice.is_paid = true;
    INVOICES.save(deps.storage, invoice_id, &invoice)?;

    Ok(Response::new()
        .add_attribute("action", "pay_invoice")
        .add_attribute("payer", info.sender.to_string())
        .add_attribute("invoice_id", invoice_id.to_string()))
}

// ===== QUERY =====

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetInvoice { invoice_id } => to_binary(&query_invoice(deps, invoice_id)?),
        QueryMsg::GetUserInvoices { user } => to_binary(&query_user_invoices(deps, user)?),
    }
}

fn query_invoice(deps: Deps, invoice_id: u64) -> StdResult<Invoice> {
    let invoice = INVOICES.load(deps.storage, invoice_id)?;
    Ok(invoice)
}

fn query_user_invoices(deps: Deps, user: String) -> StdResult<Vec<Invoice>> {
    let user_addr = deps.api.addr_validate(&user)?;
    let invoice_ids = USER_INVOICES.may_load(deps.storage, user_addr)?.unwrap_or_default();
    let invoices: Vec<Invoice> = invoice_ids
        .into_iter()
        .filter_map(|id| INVOICES.may_load(deps.storage, id).ok())
        .flatten()
        .collect();
    Ok(invoices)
}
