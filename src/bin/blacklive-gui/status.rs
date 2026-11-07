use super::App;
use eframe::egui::{self, Color32, RichText};

impl App {
    pub(crate) fn show_status(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        let color = if self.status_is_error {
            Color32::from_rgb(245, 110, 110)
        } else {
            Color32::from_rgb(120, 210, 150)
        };
        ui.label(RichText::new(&self.status).color(color).strong());

        if let Some(result) = &self.last_result {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "写入 {} / 源 {} 音符   {} 组 / {} MIDI 轨   BPM {}   长度 {} beats",
                    result.used_note_count,
                    result.source_note_count,
                    result.groups.len(),
                    result
                        .groups
                        .iter()
                        .map(|group| group.tracks.len())
                        .sum::<usize>(),
                    blacklive::format_num(result.bpm),
                    blacklive::format_num(result.clip_length_beats)
                ));
                if result.tempo_automation {
                    ui.label(format!(
                        "Tempo 自动化 {} 点",
                        result.midi_stats.tempo_map.len()
                    ));
                }
            });
            ui.horizontal_wrapped(|ui| {
                for group in &result.groups {
                    ui.label(format!("{} · {} MIDI 轨", group.name, group.tracks.len()));
                }
            });
        }

        if !self.log.is_empty() {
            ui.add_space(2.0);
            egui::ScrollArea::vertical()
                .id_salt("log_scroll")
                .max_height(150.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.log {
                        let color = if line.starts_with('!') {
                            Color32::from_rgb(245, 110, 110)
                        } else if line.starts_with('⚠') {
                            Color32::from_rgb(235, 180, 90)
                        } else {
                            Color32::GRAY
                        };
                        ui.label(RichText::new(line).color(color));
                    }
                });
        }
    }
}
