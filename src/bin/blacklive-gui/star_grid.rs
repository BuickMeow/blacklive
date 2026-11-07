use eframe::egui::{self, FontId, Rect, Ui, Vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TrackSize {
    Auto,
    Fixed(f32),
    Star(f32),
}

#[derive(Clone, Debug)]
pub(crate) struct StarGrid {
    columns: Vec<TrackSize>,
    rows: Vec<TrackSize>,
    column_gap: f32,
    row_gap: f32,
}

impl StarGrid {
    pub(crate) fn new(columns: Vec<TrackSize>, rows: Vec<TrackSize>) -> Self {
        Self {
            columns,
            rows,
            column_gap: 0.0,
            row_gap: 0.0,
        }
    }

    pub(crate) fn gaps(mut self, column_gap: f32, row_gap: f32) -> Self {
        self.column_gap = column_gap;
        self.row_gap = row_gap;
        self
    }

    pub(crate) fn cells(
        &self,
        rect: Rect,
        auto_column_widths: &[f32],
        auto_row_heights: &[f32],
    ) -> Vec<Vec<Rect>> {
        let widths = self.column_widths(rect.width(), auto_column_widths);
        let heights = self.row_heights(rect.height(), auto_row_heights);
        let mut cells = Vec::with_capacity(heights.len());
        let mut y = rect.top();
        for height in heights {
            let mut row = Vec::with_capacity(widths.len());
            let mut x = rect.left();
            for width in &widths {
                row.push(Rect::from_min_size(
                    egui::pos2(x, y),
                    Vec2::new(*width, height),
                ));
                x += width + self.column_gap;
            }
            cells.push(row);
            y += height + self.row_gap;
        }
        cells
    }

    pub(crate) fn column_widths(&self, total: f32, auto_widths: &[f32]) -> Vec<f32> {
        resolve(&self.columns, total, self.column_gap, auto_widths)
    }

    pub(crate) fn row_heights(&self, total: f32, auto_heights: &[f32]) -> Vec<f32> {
        resolve(&self.rows, total, self.row_gap, auto_heights)
    }
}

fn resolve(tracks: &[TrackSize], total: f32, gap: f32, autos: &[f32]) -> Vec<f32> {
    let mut sizes = vec![0.0; tracks.len()];
    let mut used = gap * tracks.len().saturating_sub(1) as f32;
    let mut star_weight = 0.0;
    for (index, track) in tracks.iter().enumerate() {
        match track {
            TrackSize::Auto => {
                let width = autos.get(index).copied().unwrap_or(0.0);
                sizes[index] = width;
                used += width;
            }
            TrackSize::Fixed(width) => {
                sizes[index] = *width;
                used += *width;
            }
            TrackSize::Star(weight) => {
                star_weight += weight;
            }
        }
    }
    let remaining = (total - used).max(0.0);
    if star_weight > 0.0 {
        for (index, track) in tracks.iter().enumerate() {
            if let TrackSize::Star(weight) = track {
                sizes[index] = remaining * weight / star_weight;
            }
        }
    }
    sizes
}

pub(crate) fn measure_text(ui: &Ui, text: &str, size: f32) -> f32 {
    ui.painter()
        .layout_no_wrap(
            text.to_owned(),
            FontId::proportional(size),
            egui::Color32::BLACK,
        )
        .size()
        .x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_columns_share_remaining_width_by_weight() {
        let grid = StarGrid::new(
            vec![
                TrackSize::Fixed(100.0),
                TrackSize::Star(1.0),
                TrackSize::Star(2.0),
            ],
            vec![TrackSize::Fixed(10.0)],
        );
        let widths = grid.column_widths(400.0, &[]);
        assert_eq!(widths, vec![100.0, 100.0, 200.0]);
    }

    #[test]
    fn auto_columns_take_their_measured_width() {
        let grid = StarGrid::new(
            vec![TrackSize::Auto, TrackSize::Star(1.0), TrackSize::Auto],
            vec![TrackSize::Fixed(10.0)],
        );
        let widths = grid.column_widths(400.0, &[80.0, 0.0, 120.0]);
        assert_eq!(widths, vec![80.0, 200.0, 120.0]);
    }

    #[test]
    fn cells_respect_gaps() {
        let grid = StarGrid::new(
            vec![TrackSize::Star(1.0), TrackSize::Star(1.0)],
            vec![TrackSize::Fixed(20.0), TrackSize::Fixed(30.0)],
        )
        .gaps(10.0, 4.0);
        let cells = grid.cells(
            Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(210.0, 54.0)),
            &[],
            &[],
        );
        assert_eq!(cells[0][0].width(), 100.0);
        assert_eq!(cells[0][1].left(), 110.0);
        assert_eq!(cells[1][0].top(), 24.0);
    }
}
