// ----------------------------------------------------------------------------
// convenience impl and helper for Egui
// ----------------------------------------------------------------------------
pub(super) trait UiExtension {
    // ------------------------------------------------------------------------
    fn checkbox_width(&self, text: impl Into<egui::WidgetText>) -> f32;
    fn end_row_if(&mut self, is_true: bool);
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
}
// ----------------------------------------------------------------------------
use bevy_egui::egui;
// ----------------------------------------------------------------------------
