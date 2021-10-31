use yew::prelude::*;

mod item;

enum Msg {
    AddOne,
    RemoveOne,
}

struct Model {
    // `ComponentLink` is like a reference to a component.
    // It can be used to send messages to the component
    link: ComponentLink<Self>,
    value: isize,
}

struct Root {
    link: ComponentLink<Self>,
}

impl Component for Root {
    type Message = ();
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self { link }
    }

    fn update(&mut self, _: Self::Message) -> bool {
        false
    }

    fn change(&mut self, _props: Self::Properties) -> bool {
        false
    }

    fn view(&self) -> Html {
        return html! {
            <div>
                <Model />
            </div>
        };
    }
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            value: 0,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::AddOne => {
                self.value += 1;
                // the value has changed so we need to
                // re-render for it to appear on the page
                true
            }
            Msg::RemoveOne => {
                self.value -= 1;
                true
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        // Should only return "true" if new properties are different to
        // previously received properties.
        // This component has no properties so we will always return "false".
        false
    }

    fn view(&self) -> Html {
        return html! {
            <div>
                <button onclick=self.link.callback(|_| Msg::AddOne)>{ "+1" }</button>
                <button onclick=self.link.callback(|_| Msg::RemoveOne)>{ "-1" }</button>
                <p>{ self.value }</p>
            </div>
        };
    }
}

fn main() {
    yew::start_app::<Root>();
}