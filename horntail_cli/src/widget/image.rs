use horntail::Canvas;
use image::DynamicImage;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;
use ratatui_image::StatefulImage;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

pub struct ImageViewState {
    picker: Picker,
    stateful_protocol: Option<StatefulProtocol>,
}

impl ImageViewState {
    pub fn new(picker: Picker) -> Self {
        Self {
            picker,
            stateful_protocol: None,
        }
    }

    #[inline]
    pub fn set_canvas(&mut self, canvas: Canvas) {
        self.stateful_protocol.replace(
            self.picker
                .new_resize_protocol(DynamicImage::from(canvas.image().unwrap())),
        );
    }

    #[inline]
    pub fn reset(&mut self) {
        self.stateful_protocol.take();
    }
}

pub struct ImageView;

impl ImageView {
    pub fn new() -> ImageView {
        ImageView
    }
}

impl StatefulWidget for ImageView {
    type State = ImageViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let Some(protocol) = state.stateful_protocol.as_mut() else {
            return;
        };
        StatefulImage::default().render(area, buf, protocol);
    }
}
