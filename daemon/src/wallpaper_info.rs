use std::{path::PathBuf, time::Duration};

use serde::Deserialize;

use crate::{image_picker::ImagePicker, render::Transition};

#[derive(PartialEq, Debug)]
pub struct WallpaperInfo {
    pub path: PathBuf,
    pub duration: Option<Duration>,
    pub apply_shadow: bool,
    pub sorting: Sorting,
    pub mode: BackgroundMode,
    pub drawn_images_queue_size: usize,
    pub transition_time: u32,

    /// Determines if we should show the transition between black and first
    /// wallpaper. `false` means we instantly cut to the first wallpaper,
    /// `true` means we fade from black to the first wallpaper.
    pub initial_transition: bool,
    pub transition: Transition,
}

impl Default for WallpaperInfo {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            duration: None,
            apply_shadow: false,
            sorting: Sorting::default(),
            mode: BackgroundMode::default(),
            drawn_images_queue_size: ImagePicker::DEFAULT_DRAWN_IMAGES_QUEUE_SIZE,
            transition_time: Transition::Fade {}.default_transition_time(),
            initial_transition: true,
            transition: Transition::Fade {},
        }
    }
}

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sorting {
    #[default]
    Random,
    Ascending,
    Descending,
}

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundMode {
    Stretch,
    #[default]
    Center,
    Fit,
    Tile,
}
