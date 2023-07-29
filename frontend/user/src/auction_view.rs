use std::rc::Rc;

use communication::auction::state::AuctionState;
use yew::prelude::*;

use common::{
    components::AuctionReportView,
    layout::{Container, VerticalStack},
    screens::fullscreen_message::FullscreenMsg,
};

use crate::{
    components::{
        bidding_screen::BiddingScreen,
        item_sold::{SoldToSomeoneElse, SoldToYou},
        show_item_before_bid::ShowItemBeforeBid,
    },
    AppCtx,
};

#[function_component]
pub fn AuctionView() -> Html {
    let ctx: Rc<AppCtx> = use_context().expect("no ctx found");
    let my_account = &ctx.my_account;
    match &ctx.state {
        AuctionState::WaitingForAuction => {
            html!(<FullscreenMsg message="Ждем начала аукциона..." show_reload_button={true} user_account={Some((my_account).into())}/>)
        }
        AuctionState::AuctionOver(report) => {
            html!(
            <Container>
                <VerticalStack>
                    <h1>{"Аукцион завершен"}</h1>
                    <AuctionReportView report={report.clone()} highlight_user_id={Some(my_account.id)}/>
                </VerticalStack>
            </Container>

            )
        }
        AuctionState::WaitingForItem => {
            html!(<FullscreenMsg message="Ждем, пока покажут товар..." show_reload_button={true} user_account={Some((my_account).into())}/>)
        }
        AuctionState::ShowingItemBeforeBidding(item) => {
            html!(<ShowItemBeforeBid item={item.clone()} />)
        }
        AuctionState::Bidding(bid_state) => {
            html!(<BiddingScreen bid_state={bid_state.clone()} />)
        }
        AuctionState::SoldToYou {
            item,
            sold_for,
            confirmation_code,
            contributions,
        } => {
            html!(<SoldToYou item={item.clone()} sold_for={sold_for} confirmation_code={confirmation_code.clone()} contributions={contributions.clone()} />)
        }
        AuctionState::SoldToSomeoneElse {
            item,
            sold_to,
            sold_for,
            contributions,
        } => {
            html!(<SoldToSomeoneElse item={item.clone()} sold_to={sold_to.clone()} sold_for={sold_for} contributions={contributions.clone()} />)
        }
        _ => {
            html!(<FullscreenMsg message={format!("Состояние аукциона не имплементировано: {:?}", ctx.state)} show_reload_button={true} user_account={Some((my_account).into())} />)
        }
    }
}
