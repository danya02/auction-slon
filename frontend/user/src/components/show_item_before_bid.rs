use common::{
    components::ItemDisplay,
    layout::{Container, VerticalStack},
};
use communication::auction::state::AuctionItem;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ShowItemProps {
    pub item: AuctionItem,
}

#[function_component]
pub fn ShowItemBeforeBid(props: &ShowItemProps) -> Html {
    html! {
        <Container>
            <VerticalStack>
                <h1>{"Prepare to bid on item:"}</h1>
                <ItemDisplay item={props.item.clone()} />
            </VerticalStack>
        </Container>
    }
}
