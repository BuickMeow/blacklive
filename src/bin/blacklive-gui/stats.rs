use super::App;
use eframe::egui;

impl App {
    pub(crate) fn show_midi_stats(&mut self, ui: &mut egui::Ui) {
        let Some(stats) = &self.midi_stats else {
            return;
        };
        ui.add_space(6.0);
        ui.label(format!(
            "{} 音符   PPQ {}   BPM {}   最大复音 {}   {} beats",
            stats.note_count,
            stats.ppq,
            stats
                .initial_bpm
                .map(blacklive::format_num)
                .unwrap_or_else(|| "?".into()),
            stats.max_polyphony,
            blacklive::format_num(stats.duration_beats)
        ));
        ui.horizontal_wrapped(|ui| {
            for (channel, count) in &stats.notes_per_channel {
                ui.label(format!("ch{}: {count}", channel));
            }
        });
        for channel in 0u8..=15 {
            let mut parts = Vec::new();
            if let Some(controllers) = stats.controllers.get(&channel) {
                for (controller, events) in controllers {
                    if !events.is_empty() {
                        parts.push(format!("CC{controller}×{}", events.len()));
                    }
                }
            }
            if let Some(bends) = stats.pitch_bends.get(&channel)
                && !bends.is_empty()
            {
                parts.push(format!("PB×{}", bends.len()));
            }
            if !parts.is_empty() {
                ui.label(format!("ch{channel}: {}", parts.join("  ")));
            }
        }
        if stats.tempo_map.len() > 1 {
            ui.label(format!("Tempo 点: {}", stats.tempo_map.len()));
        }
    }
}
