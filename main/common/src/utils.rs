/// see also: [`egui::ScrollArea::show_rows`]
pub fn egui_scroll_area_show_columns<R>(
    scroll_area: egui::ScrollArea,
    ui: &mut egui::Ui,
    column_width_sans_spacing: f32,
    total_columns: usize,
    add_contents: impl FnOnce(&mut egui::Ui, std::ops::Range<usize>) -> R,
) -> egui::scroll_area::ScrollAreaOutput<R> {
    let spacing = ui.spacing().item_spacing;
    let column_width_with_spacing = column_width_sans_spacing + spacing.x;
    scroll_area.show_viewport(ui, |ui, viewport| {
        ui.set_width((column_width_with_spacing * total_columns as f32 - spacing.x).max(0.0));

        let mut min_column = (viewport.min.x / column_width_with_spacing).floor() as usize;
        let mut max_column = (viewport.max.x / column_width_with_spacing).ceil() as usize + 1;
        if max_column > total_columns {
            let diff = max_column.saturating_sub(min_column);
            max_column = total_columns;
            min_column = total_columns.saturating_sub(diff);
        }

        let x_min = ui.max_rect().left() + min_column as f32 * column_width_with_spacing;
        let x_max = ui.max_rect().left() + max_column as f32 * column_width_with_spacing;

        let rect = egui::Rect::from_x_y_ranges(x_min..=x_max, ui.max_rect().y_range());

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |viewport_ui| {
            viewport_ui.skip_ahead_auto_ids(min_column); // Make sure we get consistent IDs.
            add_contents(viewport_ui, min_column..max_column)
        })
        .inner
    })
}
