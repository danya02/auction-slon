use communication::{auction::state::AuctionItem, Money, UserAccountData};
use yew::{prelude::*, virtual_dom::VNode};

#[derive(Properties, PartialEq)]
pub struct UserAccountCardProps {
    pub account: UserAccountData,
}

/// Card showing a user's name and balance
#[function_component]
pub fn UserAccountCard(props: &UserAccountCardProps) -> Html {
    html! {
        <div class="card" style="min-width: 15em;">
            <div class="card-body">
                <h5 class="card-title">{&props.account.user_name}</h5>
                <h6 class="card-subtitle mb-5">{"Balance: "}<MoneyDisplay money={props.account.balance} /> </h6>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct UserAccountTableProps {
    pub accounts: Vec<UserAccountData>,
}

/// Table showing many user names and balances
#[function_component]
pub fn UserAccountTable(props: &UserAccountTableProps) -> Html {
    html! {
        <table class="table table-sm table-dark table-striped">
            <colgroup>
                <col span="1" style="width: 70%;" />
                <col span="1" style="width: 30%;" />
            </colgroup>

            <thead>
                <tr>
                    <th scope="col">{"Name"}</th>
                    <th scope="col">{"Balance"}</th>
                </tr>
            </thead>

            <tbody class="table-group-divider">
                { for props.accounts.iter().map(|i| html!(
                    <tr>
                    <td>{&i.user_name}</td>
                    <td><MoneyDisplay money={i.balance} /></td>
                    </tr>
                    )
                )}
            </tbody>
        </table>
    }
}

#[derive(Properties, PartialEq)]
pub struct MoneyDisplayProps {
    pub money: Money,
}

/// Text span showing an amount of money, with a currency symbol.
#[function_component]
pub fn MoneyDisplay(props: &MoneyDisplayProps) -> Html {
    let currency_symbol_svg = include_str!("../../../slon-icon-filled.svg");
    let currency_symbol = VNode::from_html_unchecked(currency_symbol_svg.into());
    html! {
        <span style="color: #D4AF37"> // strong yellow / gold color
            {props.money}{" "}
            {currency_symbol}
        </span>
    }
}

#[derive(Properties, PartialEq)]
pub struct ItemDisplayProps {
    pub item: AuctionItem,
}

/// Show overview info card about an item, including its name and initial price.
#[function_component]
pub fn ItemDisplay(props: &ItemDisplayProps) -> Html {
    let item = &props.item;
    html!(
        <div class="card mb-3">
            <div class="card-body">
                <h5 class="card-title">{&item.name}</h5>
                <h6 class="card-subtitle">{"Initial price: "}<MoneyDisplay money={item.initial_price} /></h6>
            </div>
        </div>
    )
}
