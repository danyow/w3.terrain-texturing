// ----------------------------------------------------------------------------
// convenience impl and helper for Egui
// ----------------------------------------------------------------------------
pub(super) trait UiExtension {
    // ------------------------------------------------------------------------
    // helper functions
    // ------------------------------------------------------------------------
    fn checkbox_width(&self, text: impl Into<egui::WidgetText>) -> f32;
    fn end_row_if(&mut self, is_true: bool);
    // ------------------------------------------------------------------------
    // widget extensions/helper
    // ------------------------------------------------------------------------
    fn deselectable_value<Value: PartialEq>(
        &mut self,
        current_value: &mut Option<Value>,
        selected_value: Value,
        text: impl Into<egui::WidgetText>,
    ) -> egui::Response;
    // ------------------------------------------------------------------------
    fn small_text(&self, text: impl Into<String>) -> egui::RichText;
    // ------------------------------------------------------------------------
    fn small_selectable_button(&self, selected: bool, label: impl Into<String>) -> egui::Button;
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl UiExtension for egui::Ui {
    // ------------------------------------------------------------------------
    #[inline]
    fn checkbox_width(&self, text: impl Into<egui::WidgetText>) -> f32 {
        let text = text.into();
        // from Checkbox Widget
        let spacing = self.spacing();
        let icon_width = spacing.icon_width;
        let icon_spacing = spacing.icon_spacing;
        let button_padding = spacing.button_padding.x;
        let total_extra = button_padding + icon_width + icon_spacing + button_padding;

        let wrap_width = self.available_width() - total_extra;
        let text = text.into_galley(self, None, wrap_width, egui::TextStyle::Button);

        total_extra + text.size().x
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    fn end_row_if(&mut self, is_true: bool) {
        if is_true {
            self.end_row();
        }
    }
    // ------------------------------------------------------------------------
    // widget extensions
    // ------------------------------------------------------------------------
    fn deselectable_value<Value: PartialEq>(
        &mut self,
        current_value: &mut Option<Value>,
        selected_value: Value,
        text: impl Into<egui::WidgetText>,
    ) -> egui::Response {
        let already_selected = if let Some(current_value) = current_value.as_ref() {
            *current_value == selected_value
        } else {
            false
        };
        let mut response = self.selectable_label(already_selected, text);
        if response.clicked() {
            if already_selected {
                current_value.take();
            } else {
                let _ = current_value.insert(selected_value);
            }
            response.mark_changed();
        }
        response
    }
    // ------------------------------------------------------------------------
    fn small_text(&self, text: impl Into<String>) -> egui::RichText {
        egui::RichText::new(text).small()
    }
    // ------------------------------------------------------------------------
    fn small_selectable_button(&self, selected: bool, label: impl Into<String>) -> egui::Button {
        let label = egui::RichText::new(label).small();
        if selected {
            egui::Button::new(label)
                .small()
                .fill(self.style().visuals.selection.bg_fill)
        } else {
            egui::Button::new(label).small()
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
use bevy_egui::egui;
// ----------------------------------------------------------------------------
