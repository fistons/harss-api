use log::debug;

use crate::services::channels::ChannelService;
use crate::services::items::ItemService;

pub mod auth;
pub mod channels;
pub mod items;
pub mod users;

pub fn refresh(
    item_service: &ItemService,
    channel_service: &ChannelService,
    user_id: i32,
) -> Result<(), diesel::result::Error> {
    let channels = channel_service.select_all_by_user_id(user_id)?;

    for channel in channels.iter() {
        refresh_chan(item_service, channel_service, channel.id, user_id)?;
    }
    Ok(())
}

pub fn refresh_chan(
    item_service: &ItemService,
    channel_service: &ChannelService,
    channel_id: i32,
    user_id: i32,
) -> Result<(), diesel::result::Error> {
    let channel = channel_service.select_by_id_and_user_id(channel_id, user_id)?;
    debug!("Fetching {}", &channel.name);

    let content = reqwest::blocking::get(&channel.url)
        .unwrap()
        .bytes()
        .unwrap();
    let rss_channel = rss::Channel::read_from(&content[..]).unwrap();
    for item in rss_channel.items.into_iter() {
        let i = crate::model::item::NewItem::from_rss_item(item, channel.id);
        item_service.insert(i).unwrap();
    }

    Ok(())
}
