use communication::{Money, UserAccountData};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct UserAccountCardProps {
    pub account: UserAccountData,
}

/// Card showing a user's name and balance
#[function_component]
pub fn UserAccountCard(props: &UserAccountCardProps) -> Html {
    html! {
        <div class="card">
            <div class="card-body">
                <h5 class="card-title">{&props.account.user_name}</h5>
                <h6 class="card-subtitle mb-5"> <MoneyDisplay money={props.account.balance} /> </h6>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct MoneyDisplayProps {
    pub money: Money,
}

/// Text span showing an amount of money, with a currency symbol.
#[function_component]
pub fn MoneyDisplay(props: &MoneyDisplayProps) -> Html {
    html! {
        <span style="color: #D4AF37"> // strong yellow / gold color
            {props.money}
            {"¤"}  // generic currency symbol; TODO change this for the project
        </span>
    }
}
