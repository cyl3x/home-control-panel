use gtk::prelude::*;
use relm4::*;

pub struct VideoComponent {
    uri: String,
}

#[derive(Debug)]
pub enum VideoInput {}

#[relm4::component(pub)]
impl SimpleComponent for VideoComponent {
    type Init = String;
    type Input = VideoInput;
    type Output = ();

    view! {
        #[root]
        gtk::CenterBox {
            set_orientation: gtk::Orientation::Horizontal,
            set_vexpand: true,
            set_hexpand: true,

            #[wrap(Some)]
            set_center_widget: clapper_video = &clapper_gtk::Video {
            }
        }
    }

    fn init(
        uri: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { uri };
        let widgets = view_output!();

        let player = widgets.clapper_video.property::<clapper::Player>("player");
        player
            .property::<clapper::Queue>("queue")
            .add_item(&clapper::MediaItem::new(&model.uri));
        player.set_autoplay(true);
        player.set_audio_enabled(false);
        player.set_subtitles_enabled(false);

        ComponentParts { model, widgets }
    }
}