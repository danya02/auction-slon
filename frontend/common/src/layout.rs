use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PlainChildrenProps {
    #[prop_or_default]
    pub children: Children,
}

/// Wrap the items in a Bootstrap container class
#[function_component]
pub fn Container(props: &PlainChildrenProps) -> Html {
    html! {
        <div class="container">
            { for props.children.iter() }
        </div>
    }
}

/// Put the items on top of one another, filling the height of the box that this is placed in, and centering each element horizontally.
/// Good for putting things in the center of the screen.
#[function_component]
pub fn VerticalStack(props: &PlainChildrenProps) -> Html {
    // https://stackoverflow.com/a/19461564/5936187
    html! {
        <div style="height: 100%; display: flex; align-items: center; justify-content: center;">
            { for props.children.iter() }
        </div>
    }
}
