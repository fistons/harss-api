use yew::prelude::*;

pub enum ArticleMsg {}

pub struct Article {
    link: ComponentLink<Self>,
}

impl Component for Article {
    type Message = ArticleMsg;
    type Properties = ();

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        todo!()
    }

    fn update(&mut self, msg: Self::Message) -> bool {
        todo!()
    }

    fn change(&mut self, _props: Self::Properties) -> bool {
        todo!()
    }

    fn view(&self) -> Html {
        return html! {
          <div>
            
            </div>
        };
    }
}



