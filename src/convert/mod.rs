mod channels;
mod filter;
mod grouping;
mod options;
mod pipeline;
mod results;

#[cfg(test)]
mod tests;

pub(crate) use anyhow::{Context, Result, bail};
pub use channels::parse_channel_filter;
pub(crate) use filter::{FilteredNotes, filter_notes};
pub(crate) use grouping::build_groups;
pub use options::ConvertOptions;
pub use pipeline::{
    convert_midi_to_als, convert_midi_to_als_with_default_template, inspect_template,
};
pub use results::{ConversionResult, GroupResult, TrackResult};
