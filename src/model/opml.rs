use serde::Deserialize;

use crate::model::HttpNewChannel;

#[derive(Deserialize, Debug)]
pub struct Outline {
    pub r#type: Option<String>,
    pub text: Option<String>,
    #[serde(rename = "xmlUrl")]
    pub xml_url: Option<String>,
    #[serde(rename = "outline")]
    pub outlines: Option<Vec<Outline>>,
}

#[derive(Deserialize, Debug)]
pub struct Body {
    #[serde(rename = "outline")]
    pub outlines: Vec<Outline>,
}

#[derive(Deserialize, Debug)]
pub struct Opml {
    pub body: Body,
}

impl Body {
    pub fn flatten_outlines(self) -> Vec<HttpNewChannel> {
        let mut channels = vec![];

        for o in self.outlines {
            flatten_outlines(&o, &mut channels);
        }
        channels
    }
}

//FIXME Ugliest method ever
fn flatten_outlines(outline: &Outline, channels: &mut Vec<HttpNewChannel>) {
    if outline.outlines.is_some() {
        for o in outline.outlines.as_ref().unwrap().into_iter() {
            flatten_outlines(&o, channels);
        }
    }
    if outline.xml_url.is_some() {
        let channel = HttpNewChannel { name: String::from(outline.text.as_ref().unwrap()), url: String::from(outline.xml_url.as_ref().unwrap()) };
        channels.push(channel);
    }
} 