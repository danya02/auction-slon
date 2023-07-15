use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PlainChildrenProps {
    #[prop_or_default]
    pub children: Children,
    #[prop_or_default]
    pub class: Classes,
}

/// Wrap the items in a Bootstrap container class,
/// and put them in the center of the screen,
/// unless they are big enough to scroll.
#[function_component]
pub fn Container(props: &PlainChildrenProps) -> Html {
    html! {
        // https://stackoverflow.com/a/65491575/5936187
        <div
            class={classes!("container-fluid", props.class.clone())}
            style="display: flex;
          align-items: center;
          justify-content: center;
          min-height: 100vh;
          height: auto;
          flex-direction: column;">
            { for props.children.iter() }
        </div>
    }
}

/// Put the items on top of one another, and centering each element horizontally.
/// Good for putting things in the center of the screen.
#[function_component]
pub fn VerticalStack(props: &PlainChildrenProps) -> Html {
    // https://stackoverflow.com/a/19461564/5936187
    html! {
        <div style="display: flex; align-items: center; justify-content: center; flex-direction: column;" class={classes!(props.class.clone())}>
            { props.children.iter().map(|child| html!(<div class="mb-3">{child}</div>)).collect::<Html>() }
        </div>
    }
}
