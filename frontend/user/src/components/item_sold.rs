use common::{
    components::{ItemDisplay, MoneyDisplay, UserAccountCard},
    layout::{Container, VerticalStack},
};
use communication::{auction::state::AuctionItem, Money, UserAccountData};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ItemSoldToYouProps {
    pub item: AuctionItem,
    pub sold_for: Money,
    pub confirmation_code: String,

}

#[function_component]
pub fn SoldToYou(props: &ItemSoldToYouProps) -> Html {
    html! {
        <Container class="bg-success">
            <VerticalStack>
                <h1>{"Sold: "}{&props.item.name}</h1>
                <p>{"Paid: "}<MoneyDisplay money={props.sold_for} /></p>
                <p>{"Show this code to the auctioneer:"}</p>
                <h2 style="font-size: calc(100vw/0.625/4);">{&props.confirmation_code}</h2> 
                // Size calc: https://stackoverflow.com/a/31322756/5936187
            </VerticalStack>
        </Container>
    }
}


#[derive(Properties, PartialEq)]
pub struct ItemSoldToSomeoneElseProps {
    pub item: AuctionItem,
    pub sold_for: Money,
    pub sold_to: UserAccountData,

}

#[function_component]
pub fn SoldToSomeoneElse(props: &ItemSoldToSomeoneElseProps) -> Html {
    html! {
        <Container class="bg-danger">
            <VerticalStack>
                <h1>{"You did not buy this"}</h1>
                <ItemDisplay item={props.item.clone()} />
                <p>{"It was sold for "}<MoneyDisplay money={props.sold_for} /></p>
                <UserAccountCard account={props.sold_to.clone()} />
            </VerticalStack>
        </Container>
    }
}
