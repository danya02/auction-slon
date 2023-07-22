use common::{
    components::{ItemDisplay, MoneyDisplay, UserAccountCard},
    layout::{Container, VerticalStack},
};
use communication::{auction::state::AuctionItem, Money, UserAccountData};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ItemSoldToYouProps {
    pub my_account: UserAccountData,
    pub item: AuctionItem,
    pub sold_for: Money,
    pub confirmation_code: String,
    pub contributions: Vec<(UserAccountData, Money)>,
}

#[function_component]
pub fn SoldToYou(props: &ItemSoldToYouProps) -> Html {
    let mut contributions = props.contributions.clone();
    contributions.sort_by(|i, j| j.1.cmp(&i.1));
    let contributor_table = html! {
        <table class="table table-sm">
            <thead>
                <tr>
                    <th scope="col">{"Name"}</th>
                    <th scope="col">{"Money contributed"}</th>
                </tr>
            </thead>
            <tbody>
                {for contributions.iter().map(|c| html!(
                    <tr class={classes!((c.0.id==props.my_account.id).then_some("table-active"))}>
                        <td>
                            {c.0.user_name.clone()}
                        </td>
                        <td>
                            <MoneyDisplay money={c.1} />
                        </td>
                    </tr>
                ))}
            </tbody>
        </table>
    };

    html! {
        <Container class="text-bg-success">
            <VerticalStack>
                <h1>{"Sold: "}{&props.item.name}</h1>
                <p>{"Paid: "}<MoneyDisplay money={props.sold_for} /></p>
                <p>{"Show this code to the auctioneer:"}</p>
                <h2 style="font-size: calc(100vw/0.625/4);">{&props.confirmation_code}</h2>
                // Size calc: https://stackoverflow.com/a/31322756/5936187
                {contributor_table}
            </VerticalStack>
        </Container>
    }
}

#[derive(Properties, PartialEq)]
pub struct ItemSoldToSomeoneElseProps {
    pub my_account: UserAccountData,
    pub item: AuctionItem,
    pub sold_for: Money,
    pub sold_to: UserAccountData,
    pub contributions: Vec<(UserAccountData, Money)>,
}

#[function_component]
pub fn SoldToSomeoneElse(props: &ItemSoldToSomeoneElseProps) -> Html {
    let mut contributions = props.contributions.clone();
    contributions.sort_by(|i, j| j.1.cmp(&i.1));
    let contributor_table = html! {
        <table class="table table-sm">
            <thead>
                <tr>
                    <th scope="col">{"Name"}</th>
                    <th scope="col">{"Money contributed"}</th>
                </tr>
            </thead>
            <tbody>
                {for contributions.iter().map(|c| html!(
                    <tr class={classes!((c.0.id==props.my_account.id).then_some("table-active"))}>
                        <td>
                            {c.0.user_name.clone()}
                        </td>
                        <td>
                            <MoneyDisplay money={c.1} />
                        </td>
                    </tr>
                ))}
            </tbody>
        </table>
    };

    if props
        .contributions
        .iter()
        .any(|i| i.0.id == props.my_account.id)
    {
        html! {
            <Container class="text-bg-warning">
                <VerticalStack>
                    <h1>{"You helped buy this"}</h1>
                    <ItemDisplay item={props.item.clone()} />
                    <div class="alert alert-info">
                        {"It was sold for "}<MoneyDisplay money={props.sold_for} />
                    </div>
                    <UserAccountCard account={props.sold_to.clone()} />
                    {contributor_table}
                </VerticalStack>
            </Container>
        }
    } else {
        html! {
            <Container class="text-bg-danger">
                <VerticalStack>
                    <h1>{"You did not buy this"}</h1>
                    <ItemDisplay item={props.item.clone()} />
                    <div class="alert alert-info">
                        {"It was sold for "}<MoneyDisplay money={props.sold_for} />
                    </div>
                    <UserAccountCard account={props.sold_to.clone()} />
                    {contributor_table}
                </VerticalStack>
            </Container>
        }
    }
}
