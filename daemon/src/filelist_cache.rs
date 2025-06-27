use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use color_eyre::eyre::{eyre, Result, WrapErr};
use hotwatch::Hotwatch;
use log::error;
use smithay_client_toolkit::reexports::calloop::{self, ping::Ping, LoopHandle};
use walkdir::WalkDir;

use crate::{wallpaper_info::Recursive, wpaperd::Wpaperd};

#[derive(Debug)]
struct Filelist {
    path: PathBuf,
    recursive: Recursive,
    filelist: Arc<Vec<PathBuf>>,
    outdated: Arc<AtomicBool>,
}

impl Filelist {
    fn new(path: &Path, recursive: Recursive) -> Self {
        let mut res = Self {
            path: path.to_path_buf(),
            recursive,
            filelist: Arc::new(Vec::new()),
            outdated: Arc::new(AtomicBool::new(true)),
        };
        res.populate();
        res
    }

    fn populate(&mut self) {
        self.filelist = Arc::new(
            WalkDir::new(&self.path)
                .max_depth(if self.recursive == Recursive::Off {
                    1
                } else {
                    usize::MAX
                })
                .follow_links(true)
                .sort_by_file_name()
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    if let Some(guess) = new_mime_guess::from_path(e.path()).first() {
                        guess.type_() == "image"
                    } else {
                        false
                    }
                })
                .map(|e| e.path().to_path_buf())
                .collect(),
        );
        self.outdated.store(false, Ordering::Relaxed);
    }
}

pub struct FilelistCache {
    cache: Vec<Filelist>,
}

impl FilelistCache {
    pub fn new(
        paths: Vec<(PathBuf, Recursive)>,
        hotwatch: &mut Hotwatch,
        event_loop_handle: LoopHandle<Wpaperd>,
    ) -> Result<(Ping, Self)> {
        let (ping, ping_source) =
            calloop::ping::make_ping().wrap_err("Failed to initialize a calloop::ping::Ping")?;

        let mut filelist_cache = Self { cache: Vec::new() };
        filelist_cache.update_paths(paths, hotwatch, ping.clone());
        event_loop_handle
            .insert_source(ping_source, move |_, _, wpaperd| {
                wpaperd.filelist_cache.borrow_mut().update_cache();
            })
            .map_err(|e| eyre!("{e:?}"))
            .wrap_err("Failed to insert the filelist cache watcher in the event loop")?;

        Ok((ping, filelist_cache))
    }

    pub fn get(&self, path: &Path, recursive: Recursive) -> Arc<Vec<PathBuf>> {
        debug_assert!(path.is_dir());
        self.cache
            .iter()
            .find(|filelist| filelist.path == path && filelist.recursive == recursive)
            .expect("Path passed to Filelist::get must have been cached")
            .filelist
            .clone()
    }

    /// paths must be sorted
    pub fn update_paths(
        &mut self,
        paths: Vec<(PathBuf, Recursive)>,
        hotwatch: &mut Hotwatch,
        event_loop_ping: Ping,
    ) {
        self.cache.retain(|filelist| {
            let path_exists = filelist.path.exists();
            if paths
                .iter()
                .any(|(path, recursive)| &filelist.path == path && filelist.recursive == *recursive)
                && path_exists
            {
                true
            } else {
                // Stop watching paths that have been removed
                // Check that it exists before
                if path_exists {
                    if let Err(err) = hotwatch.unwatch(&filelist.path).wrap_err_with(|| {
                        format!("Failed to unwatch changes for file {:?}", &filelist.path)
                    }) {
                        error!("{err:?}");
                    }
                }
                // and remove them from the vec
                false
            }
        });

        for (path, recursive) in paths {
            if !self.cache.iter().any(|filelist| filelist.path == path) {
                // Skip paths that don't exists and files
                if !path.exists() || !path.is_dir() {
                    continue;
                }
                let filelist = Filelist::new(&path, recursive);
                let outdated = filelist.outdated.clone();
                self.cache.push(filelist);
                let ping_clone = event_loop_ping.clone();
                if let Err(err) = hotwatch
                    .watch(&path, move |event| match event.kind {
                        hotwatch::EventKind::Create(_)
                        | hotwatch::EventKind::Remove(_)
                        | hotwatch::EventKind::Modify(_) => {
                            // We could manually update the list of files with the information
                            // we get here, but the inotify on linux is not reliable,
                            // so we prefer to always trigger an update and just reload
                            // the entire list
                            // See: https://github.com/notify-rs/notify/issues/412
                            outdated.store(true, Ordering::Release);
                            ping_clone.ping();
                        }
                        _ => {}
                    })
                    .wrap_err_with(|| format!("Failed to watch for changes for file {:?}", &path))
                {
                    error!("{err:?}");
                }
            }
        }

        self.update_cache();
    }

    pub fn update_cache(&mut self) {
        for filelist in &mut self.cache {
            if filelist.outdated.load(std::sync::atomic::Ordering::Relaxed) {
                filelist.populate();
            }
        }
    }
}
