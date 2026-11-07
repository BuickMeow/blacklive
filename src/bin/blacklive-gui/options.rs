use super::App;
use eframe::egui;

impl App {
    pub(crate) fn show_options(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        egui::Grid::new("options")
            .num_columns(4)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("通道");
                ui.add(egui::TextEdit::singleline(&mut self.channels).desired_width(150.0));
                ui.checkbox(&mut self.include_drum_channel, "包含 GM 鼓通道");
                ui.end_row();

                ui.label("移调");
                ui.add(egui::DragValue::new(&mut self.transpose).suffix(" 半音"));
                ui.label("BPM");
                ui.add(egui::TextEdit::singleline(&mut self.bpm_text).desired_width(90.0));
                ui.end_row();

                ui.label("最短时值");
                ui.add(
                    egui::DragValue::new(&mut self.min_note_beats)
                        .speed(0.001)
                        .suffix(" beats"),
                );
                ui.label("尾部留白");
                ui.add(
                    egui::DragValue::new(&mut self.tail_beats)
                        .speed(0.01)
                        .suffix(" beats"),
                );
                ui.end_row();

                ui.label("Arrangement 起点");
                ui.add(
                    egui::DragValue::new(&mut self.start_beats)
                        .speed(0.25)
                        .suffix(" beats"),
                );
                ui.checkbox(&mut self.trim_same_pitch, "同音高重叠截短前音");
                ui.end_row();

                ui.label("Clip 名称");
                ui.add(egui::TextEdit::singleline(&mut self.clip_name).desired_width(150.0));
                ui.checkbox(&mut self.split_source_tracks, "按源轨道拆分");
                ui.end_row();

                ui.label("");
                ui.checkbox(
                    &mut self.strip_instrument_effects,
                    "关闭乐器内 Reverb/Delay",
                );
                ui.end_row();
            });
    }
}
