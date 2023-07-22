use communication::{
    auction::state::{AuctionItem, AuctionReport, Sponsorship},
    Money, UserAccountData,
};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlInputElement;
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
    pub users: Vec<UserAccountData>,
    pub sponsorships: Vec<Sponsorship>,
    pub action_col_cb: Option<Callback<UserAccountData, Html>>,
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
                    <th scope="col">{"Available balance"}</th>
                    {if props.action_col_cb.is_some() {html!(<th scope="col">{"Actions"}</th>)} else {html!()}}
                </tr>
            </thead>

            <tbody class="table-group-divider">
                { for props.accounts.iter().map(|i| html!(
                    <tr>
                    <td>{&i.user_name}</td>
                    <td><MoneyDisplay money={Sponsorship::resolve_available_balance(i.id, &props.users, &props.sponsorships)} /></td>
                    {if props.action_col_cb.is_some() {
                        let html = props.action_col_cb.as_ref().unwrap().emit(i.clone());
                        html!(<td>{html}</td>)
                    } else {html!()}}
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

#[derive(Properties, PartialEq)]
pub struct AuctionReportViewProps {
    pub report: AuctionReport,
    pub highlight_user_id: Option<i64>,
}

/// Show an auction report in either a user-oriented or item-oriented way,
/// optionally highlighting a particular user's rows.
#[function_component]
pub fn AuctionReportView(props: &AuctionReportViewProps) -> Html {
    #[derive(Clone, Copy)]
    enum Tabs {
        UserFirst,
        ItemFirst,
    }
    use Tabs::*;

    let current_tab = use_state(|| UserFirst);

    let set_user_cb = {
        let current_tab = current_tab.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            current_tab.set(UserFirst);
        })
    };

    let set_item_cb = {
        let current_tab = current_tab.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            current_tab.set(ItemFirst);
        })
    };

    let current_tab = *current_tab;

    let info_table = match current_tab {
        UserFirst => {
            let mut rows = vec![];
            for user in &props.report.members {
                let user_id = user.id;
                // For each user, figure out which items they purchased.
                let items_bought: Vec<_> = props
                    .report
                    .items
                    .iter()
                    .filter(|i| match &i.state {
                        communication::ItemStateValue::Sellable => false,
                        communication::ItemStateValue::AlreadySold { buyer, .. } => {
                            buyer.id == user_id
                        }
                    })
                    .collect();

                // If there are none, emit a special case row with the empty symbol.
                if items_bought.is_empty() {
                    rows.push(html!(
                        // Highlight if: `highlight_user_id` is provided, and is equal to the current user's ID.
                        <tr class={classes!(props.highlight_user_id.and_then(|i| (i==user_id).then_some("table-active")))}>
                            <th scope="row">{&user.user_name}</th>
                            <td><MoneyDisplay money={user.balance} /></td>
                            <td colspan=2 style="text-align: center;">{"∅"}</td> // Empty set symbol U+2205
                        </tr>
                    ));
                    continue;
                }

                // If there are items, then the first row will contain the name and balance, and be rowspan'd to the number of items.
                let first_item = items_bought.first().unwrap();
                rows.push(html!(
                    // Highlight if: `highlight_user_id` is provided, and is equal to the current user's ID.
                    <tr class={classes!(props.highlight_user_id.and_then(|i| (i==user_id).then_some("table-active")))}>
                        <th scope="row" rowspan={items_bought.len().to_string()}>{&user.user_name}</th>
                        <td rowspan={items_bought.len().to_string()}><MoneyDisplay money={user.balance} /></td>
                        <td>{&first_item.item.name}</td>
                        <td><MoneyDisplay money={first_item.state.get_sale_price().unwrap()} /></td>
                    </tr>
                ));

                // The other rows will contain only the item and price, as the name and balance are rowspanned.
                for item in items_bought.iter().skip(1) {
                    rows.push(html!(
                        // Highlight if: `highlight_user_id` is provided, and is equal to the current user's ID.
                        <tr class={classes!(props.highlight_user_id.and_then(|i| (i==user_id).then_some("table-active")))}>
                            <td>{&item.item.name}</td>
                            <td><MoneyDisplay money={item.state.get_sale_price().unwrap()} /></td>
                        </tr>
                    ));
                }
            }
            html! {
                <table class="table table-dark">
                    <thead>
                        <tr>
                            <th scope="col">{"User name"}</th>
                            <th scope="col">{"Final balance"}</th>
                            <th scope="col">{"Purchased item"}</th>
                            <th scope="col">{"Purchased price"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for rows}
                    </tbody>
                </table>
            }
        }
        ItemFirst => {
            let mut rows = vec![];
            for item_state in &props.report.items {
                let do_highlight = match &item_state.state {
                    communication::ItemStateValue::Sellable => false,
                    communication::ItemStateValue::AlreadySold { buyer, .. } => {
                        Some(buyer.id) == props.highlight_user_id
                    }
                };
                let who_bought = match &item_state.state {
                    communication::ItemStateValue::Sellable => html! {
                            // Nobody bought this, so draw a null symbol
                            <td colspan=2 style="text-align: center;">{"∅"}</td> // Empty set symbol U+2205
                    },
                    communication::ItemStateValue::AlreadySold { buyer, sale_price } => html! {
                        <>
                            <td>{&buyer.user_name}</td>
                            <td><MoneyDisplay money={sale_price} /></td>
                        </>
                    },
                };

                let row = html!(
                    <tr class={classes!(do_highlight.then_some("table-active"))}>
                        <th scope="row">{&item_state.item.name}</th>
                        <td><MoneyDisplay money={item_state.item.initial_price} /></td>
                        {who_bought}
                    </tr>
                );
                rows.push(row);
            }

            html! {
                <table class="table table-dark">
                    <thead>
                        <tr>
                            <th scope="col">{"Item name"}</th>
                            <th scope="col">{"Initial price"}</th>
                            <th scope="col">{"Purchased by user"}</th>
                            <th scope="col">{"Purchased price"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for rows}
                    </tbody>
                </table>
            }
        }
    };

    html! {
        <div>
            <ul class="nav nav-tabs">
                <li class="nav-item">
                    <a onclick={set_user_cb} class={classes!("nav-link", matches!(current_tab, UserFirst).then_some("active"))}>{"By user"}</a>
                </li>
                <li class="nav-item">
                    <a onclick={set_item_cb} class={classes!("nav-link", matches!(current_tab, ItemFirst).then_some("active"))}>{"By item"}</a>
                </li>
            </ul>

            {info_table}
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct TextInputProps {
    /// When this changes, the input is cleared and replaced with this,
    /// and the dirty flag is cleared.
    pub prefill_value: AttrValue,

    /// This is called when an input is completed (onchange event).
    /// Not called for each keystroke (oninput).
    /// Also, the dirty flag isn't cleared when this is called.
    pub onchange: Callback<String>,
}

#[function_component]
pub fn TextInput(props: &TextInputProps) -> Html {
    let current_value = use_state(|| props.prefill_value.to_string());
    let is_dirty = use_state(|| false);

    {
        let current_value = current_value.clone();
        let is_dirty = is_dirty.clone();
        use_effect_with_deps(
            move |new_value| {
                current_value.set(new_value.to_string());
                is_dirty.set(false);
            },
            props.prefill_value.clone(),
        );
    }

    let oninput = {
        let is_dirty = is_dirty.clone();
        let current_value = current_value.clone();
        Callback::from(move |e: InputEvent| {
            let event: Event = e.dyn_into().unwrap_throw();
            let event_target = event.target().unwrap_throw();
            let target: HtmlInputElement = event_target.dyn_into().unwrap_throw();
            current_value.set(target.value());
            is_dirty.set(true);
        })
    };

    let onchange = {
        let onchange = props.onchange.clone();
        let current_value = current_value.clone();

        let is_dirty = is_dirty.clone();
        Callback::from(move |_e: Event| {
            onchange.emit((*current_value).clone());
            is_dirty.set(false);
        })
    };
    html!(
        <input
            type="text" value={(*current_value).clone()}
            class={classes!("form-control", is_dirty.then_some("border-warning"))}
            {onchange}
            {oninput} />
    )
}

#[derive(Properties, PartialEq)]
pub struct NumberInputProps {
    /// When this changes, the input is cleared and replaced with this,
    /// and the dirty flag is cleared.
    pub prefill_value: AttrValue,

    /// This is called when an input is completed (onchange event).
    /// Not called for each keystroke (oninput).
    /// Also, the dirty flag isn't cleared when this is called.
    pub onchange: Callback<String>,

    /// Minimum number value
    pub min: AttrValue,

    /// Maximum number value
    pub max: AttrValue,

    /// Number step value
    pub step: AttrValue,
}

#[function_component]
pub fn NumberInput(props: &NumberInputProps) -> Html {
    let current_value = use_state(|| props.prefill_value.to_string());
    let is_dirty = use_state(|| false);

    {
        let current_value = current_value.clone();
        let is_dirty = is_dirty.clone();
        use_effect_with_deps(
            move |new_value| {
                current_value.set(new_value.to_string());
                is_dirty.set(false);
            },
            props.prefill_value.clone(),
        );
    }

    let oninput = {
        let is_dirty = is_dirty.clone();
        let current_value = current_value.clone();
        Callback::from(move |e: InputEvent| {
            let event: Event = e.dyn_into().unwrap_throw();
            let event_target = event.target().unwrap_throw();
            let target: HtmlInputElement = event_target.dyn_into().unwrap_throw();
            current_value.set(target.value());
            is_dirty.set(true);
        })
    };

    let onchange = {
        let onchange = props.onchange.clone();
        let current_value = current_value.clone();

        let is_dirty = is_dirty.clone();
        Callback::from(move |_e: Event| {
            onchange.emit((*current_value).clone());
            is_dirty.set(false);
        })
    };

    html!(
        <input
            type="number" value={(*current_value).clone()}
            class={classes!("form-control", is_dirty.then_some("border-warning"))}
            {onchange}
            {oninput}
            min={&props.min}
            max={&props.max}
            step={&props.step} />
    )
}
