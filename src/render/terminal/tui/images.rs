//! Inline image loading for the TUI.
//!
//! Decodes local images and keeps the decoded pixels so a band can be scrolled
//! through: the widget draws one terminal-row tile at a time (ratatui-image
//! can't clip a partially scrolled image, so we crop the source ourselves and
//! cache tiny row protocols). Protocols target
//! Kitty / iTerm2 / Sixel where the terminal supports them, halfblocks else.

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use image::{DynamicImage, imageops::FilterType};
use ratatui::layout::{Rect, Size};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{Resize, ResizeEncodeRender};

/// A decoded image plus its source pixel dimensions. The pixels are retained so
/// a tall band can be cropped to whatever vertical slice is currently visible.
pub struct Loaded {
    image: Arc<DynamicImage>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SliceKey {
    src: String,
    placement_line: u16,
    row: u16,
    band_rows: u16,
    area_width: u16,
    generation: u64,
}

/// The protocol for one visible terminal-row tile. A recency cache keeps nearby
/// rows warm without retaining every row of a tall image forever.
struct SliceProto {
    proto: StatefulProtocol,
}

struct SliceRequest {
    key: SliceKey,
    image: Arc<DynamicImage>,
    y0: u32,
    y1: u32,
    size: Size,
    epoch: u64,
}

struct SliceReady {
    key: SliceKey,
    proto: Option<StatefulProtocol>,
    epoch: u64,
}

enum SliceMsg {
    Request(SliceRequest),
    Cancel { key: SliceKey, epoch: u64 },
    CancelBefore(u64),
}

struct SliceWorker {
    txs: Vec<Sender<SliceMsg>>,
    rx: Receiver<SliceReady>,
}

impl SliceWorker {
    fn send(&self, request: SliceRequest) -> Result<(), mpsc::SendError<SliceRequest>> {
        let idx = request.key.worker_index(self.txs.len());
        self.txs[idx]
            .send(SliceMsg::Request(request))
            .map_err(|err| match err.0 {
                SliceMsg::Request(request) => mpsc::SendError(request),
                SliceMsg::Cancel { .. } | SliceMsg::CancelBefore(_) => unreachable!("sent request"),
            })
    }

    fn cancel_before(&self, epoch: u64) {
        for tx in &self.txs {
            let _ = tx.send(SliceMsg::CancelBefore(epoch));
        }
    }

    fn cancel(&self, key: SliceKey, epoch: u64) {
        let idx = key.worker_index(self.txs.len());
        let _ = self.txs[idx].send(SliceMsg::Cancel { key, epoch });
    }
}

impl SliceKey {
    fn worker_index(&self, workers: usize) -> usize {
        debug_assert!(workers > 0);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        usize::try_from(hasher.finish()).unwrap_or(0) % workers
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageView {
    pub scroll: u16,
    pub height: u16,
    pub width: u16,
}

/// Where a loaded image sits in the (reserved) content flow.
#[derive(Debug, Clone)]
pub struct Placement {
    pub src: String,
    pub line: u16,
    pub rows: u16,
}

/// Loads and caches inline images, keyed by source path.
pub struct ImageStore {
    picker: Option<Picker>,
    base_dir: Option<PathBuf>,
    cache: HashMap<String, Option<Loaded>>,
    slices: HashMap<SliceKey, SliceProto>,
    slice_order: VecDeque<SliceKey>,
    pending: HashMap<SliceKey, u64>,
    wanted: HashSet<SliceKey>,
    visible: HashSet<SliceKey>,
    warming: HashSet<SliceKey>,
    worker: Option<SliceWorker>,
    generation: u64,
    request_epoch: u64,
    active_view: Option<ImageView>,
    /// Terminal cell size in pixels, for sizing reserved row bands.
    cell: (u32, u32),
}

impl ImageStore {
    pub fn new(picker: Option<Picker>, base_dir: Option<PathBuf>) -> Self {
        let cell = picker.as_ref().map_or((8, 16), |p| {
            let fs = p.font_size();
            (u32::from(fs.width.max(1)), u32::from(fs.height.max(1)))
        });
        let worker = picker.as_ref().map(spawn_slice_workers);
        Self {
            picker,
            base_dir,
            cache: HashMap::new(),
            slices: HashMap::new(),
            slice_order: VecDeque::new(),
            pending: HashMap::new(),
            wanted: HashSet::new(),
            visible: HashSet::new(),
            warming: HashSet::new(),
            worker,
            generation: 0,
            request_epoch: 0,
            active_view: None,
            cell,
        }
    }

    /// Whether a graphics-capable picker is available.
    pub fn enabled(&self) -> bool {
        self.picker.is_some()
    }

    /// Terminal cell size in pixels.
    pub fn cell(&self) -> (u32, u32) {
        self.cell
    }

    /// Drop cached images and slice protocols (e.g. on live reload, where the
    /// underlying files may have changed).
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.slices.clear();
        self.slice_order.clear();
        self.pending.clear();
        self.generation = self.generation.wrapping_add(1);
        self.cancel_pending_requests();
    }

    /// Drop generated rasters while preserving decoded document images.
    pub fn clear_generated(&mut self) {
        self.cache
            .retain(|key, _loaded| !key.starts_with(GENERATED_KEY_PREFIX));
        self.slices.clear();
        self.slice_order.clear();
        self.pending.clear();
        self.generation = self.generation.wrapping_add(1);
        self.cancel_pending_requests();
    }

    /// Ensure a generated image (e.g. a mermaid diagram) is cached under `key`,
    /// building it on first request.
    pub fn ensure_generated(
        &mut self,
        key: &str,
        build: impl FnOnce() -> Option<DynamicImage>,
    ) -> Option<&mut Loaded> {
        if !self.cache.contains_key(key) {
            let loaded = self.picker.as_ref().and_then(|_picker| {
                let image = build()?;
                Some(Loaded {
                    width: image.width(),
                    height: image.height(),
                    image: Arc::new(image),
                })
            });
            self.cache.insert(key.to_string(), loaded);
        }
        self.cache.get_mut(key).and_then(Option::as_mut)
    }

    pub fn begin_frame(&mut self, view: ImageView) {
        self.wanted.clear();
        self.visible.clear();
        if self.active_view == Some(view) {
            return;
        }
        self.active_view = Some(view);
        self.request_epoch = self.request_epoch.wrapping_add(1);
    }

    pub fn finish_frame(&mut self) {
        let Some(worker) = self.worker.as_ref() else {
            self.pending.clear();
            return;
        };
        let canceled: Vec<(SliceKey, u64)> = self
            .pending
            .iter()
            .filter(|(key, _epoch)| !self.wanted.contains(*key) && !self.warming.contains(*key))
            .map(|(key, epoch)| (key.clone(), *epoch))
            .collect();
        for (key, epoch) in canceled {
            self.pending.remove(&key);
            worker.cancel(key, epoch);
        }
    }

    pub fn poll_ready(&mut self) -> bool {
        let mut changed = false;
        let Some(worker) = self.worker.as_ref() else {
            return false;
        };
        let ready: Vec<SliceReady> = worker.rx.try_iter().collect();
        for item in ready {
            if self.pending.get(&item.key) == Some(&item.epoch) {
                self.pending.remove(&item.key);
            }
            let warming = self.warming.remove(&item.key);
            if item.key.generation != self.generation {
                continue;
            }
            let Some(proto) = item.proto else {
                continue;
            };
            if !self.cache.contains_key(&item.key.src) {
                continue;
            }
            let wanted = self.wanted.contains(&item.key);
            if item.epoch != self.request_epoch && !wanted && !warming {
                continue;
            }
            let visible = self.visible.contains(&item.key);
            self.insert_slice(item.key, proto);
            changed |= visible;
        }
        changed
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Build or reuse the protocol for one terminal row of a `band_rows`-tall band.
    /// `None` means the exact row is still preparing or the source cannot render.
    pub fn row_protocol(
        &mut self,
        src: &str,
        placement_line: u16,
        row: u16,
        band_rows: u16,
        area: Rect,
    ) -> Option<&mut StatefulProtocol> {
        let request = self.build_request(src, placement_line, row, band_rows, area.width)?;
        let key = request.key.clone();
        self.wanted.insert(key.clone());
        self.visible.insert(key.clone());
        if self.slices.contains_key(&key) {
            self.remember_slice(&key);
            return self.slices.get_mut(&key).map(|s| &mut s.proto);
        }
        self.enqueue_request(request);
        None
    }

    pub fn prefetch_row(
        &mut self,
        src: &str,
        placement_line: u16,
        row: u16,
        band_rows: u16,
        area_width: u16,
    ) {
        let Some(request) = self.build_request(src, placement_line, row, band_rows, area_width)
        else {
            return;
        };
        let key = request.key.clone();
        self.wanted.insert(key.clone());
        if self.slices.contains_key(&key) {
            self.remember_slice(&key);
            return;
        }
        self.enqueue_request(request);
    }

    pub fn warm_rows(
        &mut self,
        src: &str,
        placement_line: u16,
        start_row: u16,
        rows: u16,
        band_rows: u16,
        area_width: u16,
    ) {
        let end = start_row.saturating_add(rows).min(band_rows);
        for row in start_row..end {
            let Some(request) = self.build_request(src, placement_line, row, band_rows, area_width)
            else {
                continue;
            };
            let key = request.key.clone();
            if self.slices.contains_key(&key) {
                self.remember_slice(&key);
                continue;
            }
            self.warming.insert(key);
            self.enqueue_request(request);
        }
    }

    /// Get a cached image (loading it on first request). `None` if the source
    /// is remote-blocked, missing, jailed, or undecodable.
    pub fn get(&mut self, src: &str) -> Option<&mut Loaded> {
        if !self.cache.contains_key(src) {
            let loaded = self.load(src);
            self.cache.insert(src.to_string(), loaded);
        }
        self.cache.get_mut(src).and_then(Option::as_mut)
    }

    fn load(&self, src: &str) -> Option<Loaded> {
        // Only load when a graphics protocol is available to draw it.
        self.picker.as_ref()?;
        let bytes = if src.starts_with("http://") || src.starts_with("https://") {
            fetch_remote(src)?
        } else {
            let path = resolve(src, self.base_dir.as_deref())?;
            std::fs::read(path).ok()?
        };
        let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .ok()?;
        // Bound decode work to guard against decompression bombs from untrusted
        // documents.
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_IMAGE_DIM);
        limits.max_image_height = Some(MAX_IMAGE_DIM);
        limits.max_alloc = Some(MAX_IMAGE_ALLOC);
        reader.limits(limits);

        let image = reader.decode().ok()?;
        Some(Loaded {
            width: image.width(),
            height: image.height(),
            image: Arc::new(image),
        })
    }

    fn build_request(
        &self,
        src: &str,
        placement_line: u16,
        row: u16,
        band_rows: u16,
        area_width: u16,
    ) -> Option<SliceRequest> {
        let picker = self.picker.as_ref()?;
        let loaded = self.cache.get(src).and_then(Option::as_ref)?;
        let render_size = Resize::Fit(None).size_for(
            &loaded.image,
            picker.font_size(),
            Size::new(area_width, band_rows),
        );
        if render_size.width == 0 || render_size.height == 0 || row >= render_size.height {
            return None;
        }
        let (y0, y1) = slice_bounds(loaded.height, row, 1, render_size.height)?;
        Some(SliceRequest {
            key: SliceKey {
                src: src.to_string(),
                placement_line,
                row,
                band_rows: render_size.height,
                area_width: render_size.width,
                generation: self.generation,
            },
            image: Arc::clone(&loaded.image),
            y0,
            y1,
            size: Size::new(render_size.width, 1),
            epoch: self.request_epoch,
        })
    }

    fn enqueue_request(&mut self, request: SliceRequest) {
        let key = request.key.clone();
        if self.pending.contains_key(&key) {
            return;
        }
        if let Some(worker) = self.worker.as_ref() {
            if worker.send(request).is_err() {
                self.pending.remove(&key);
                self.warming.remove(&key);
            } else {
                self.pending.insert(key, self.request_epoch);
            }
        }
    }

    fn insert_slice(&mut self, key: SliceKey, proto: StatefulProtocol) {
        if !self.slices.contains_key(&key) {
            while self.slices.len() >= MAX_SLICE_PROTOS {
                let Some(oldest) = self.slice_order.pop_front() else {
                    break;
                };
                self.slices.remove(&oldest);
            }
        }
        self.remember_slice(&key);
        self.slices.insert(key, SliceProto { proto });
    }

    fn remember_slice(&mut self, key: &SliceKey) {
        self.slice_order.retain(|cached| cached != key);
        self.slice_order.push_back(key.clone());
    }

    fn cancel_pending_requests(&mut self) {
        self.request_epoch = self.request_epoch.wrapping_add(1);
        self.pending.clear();
        self.wanted.clear();
        self.visible.clear();
        self.warming.clear();
        self.active_view = None;
        if let Some(worker) = self.worker.as_ref() {
            worker.cancel_before(self.request_epoch);
        }
    }
}

fn spawn_slice_workers(picker: &Picker) -> SliceWorker {
    let (ready_tx, ready_rx) = mpsc::channel::<SliceReady>();
    let workers = thread::available_parallelism()
        .map_or(2, |parallel| parallel.get().clamp(2, MAX_SLICE_WORKERS));
    let mut txs = Vec::with_capacity(workers);
    for idx in 0..workers {
        let (tx, rx) = mpsc::channel::<SliceMsg>();
        txs.push(tx);
        let ready_tx = ready_tx.clone();
        let picker = picker.clone();
        let _ = thread::Builder::new()
            .name(format!("silkprint-tui-images-{idx}"))
            .spawn(move || slice_worker_loop(&picker, &rx, &ready_tx));
    }
    SliceWorker { txs, rx: ready_rx }
}

fn slice_worker_loop(picker: &Picker, rx: &Receiver<SliceMsg>, ready_tx: &Sender<SliceReady>) {
    let mut min_epoch = 0;
    let mut canceled = HashSet::new();
    let mut queued = VecDeque::new();
    while let Some(mut request) = queued
        .pop_front()
        .or_else(|| recv_slice_request(rx, &mut min_epoch))
    {
        drain_slice_requests(
            rx,
            ready_tx,
            &mut queued,
            &mut request,
            &mut min_epoch,
            &mut canceled,
        );
        let cancel_key = (request.key.clone(), request.epoch);
        if request.epoch < min_epoch || canceled.remove(&cancel_key) {
            send_canceled(ready_tx, request.key, request.epoch);
            continue;
        }
        let proto = prepare_slice(picker, &request);
        if ready_tx
            .send(SliceReady {
                key: request.key,
                proto,
                epoch: request.epoch,
            })
            .is_err()
        {
            break;
        }
    }
}

fn recv_slice_request(rx: &Receiver<SliceMsg>, min_epoch: &mut u64) -> Option<SliceRequest> {
    loop {
        match rx.recv().ok()? {
            SliceMsg::Request(request) => return Some(request),
            SliceMsg::Cancel { .. } => {}
            SliceMsg::CancelBefore(epoch) => *min_epoch = (*min_epoch).max(epoch),
        }
    }
}

fn drain_slice_requests(
    rx: &Receiver<SliceMsg>,
    ready_tx: &Sender<SliceReady>,
    queued: &mut VecDeque<SliceRequest>,
    request: &mut SliceRequest,
    min_epoch: &mut u64,
    canceled: &mut HashSet<(SliceKey, u64)>,
) {
    while let Ok(msg) = rx.try_recv() {
        let newer = match msg {
            SliceMsg::Request(request) => request,
            SliceMsg::Cancel { key, epoch } => {
                if let Some(pos) = queued
                    .iter()
                    .position(|queued| queued.key == key && queued.epoch == epoch)
                {
                    let Some(old) = queued.remove(pos) else {
                        continue;
                    };
                    send_canceled(ready_tx, old.key, old.epoch);
                } else if request.key == key && request.epoch == epoch {
                    canceled.insert((key, epoch));
                } else {
                    // The row already finished before the cancel reached this worker.
                }
                continue;
            }
            SliceMsg::CancelBefore(epoch) => {
                *min_epoch = (*min_epoch).max(epoch);
                continue;
            }
        };
        if newer.epoch < *min_epoch {
            send_canceled(ready_tx, newer.key, newer.epoch);
            continue;
        }
        if same_slice_stream(&newer.key, &request.key) {
            let old = std::mem::replace(request, newer);
            send_canceled(ready_tx, old.key, old.epoch);
        } else if let Some(pos) = queued
            .iter()
            .position(|queued| same_slice_stream(&queued.key, &newer.key))
        {
            let Some(old) = queued.remove(pos) else {
                continue;
            };
            send_canceled(ready_tx, old.key, old.epoch);
            queued.push_back(newer);
        } else {
            queued.push_back(newer);
        }
    }
}

fn same_slice_stream(a: &SliceKey, b: &SliceKey) -> bool {
    a == b
}

fn send_canceled(ready_tx: &Sender<SliceReady>, key: SliceKey, epoch: u64) {
    let _ = ready_tx.send(SliceReady {
        key,
        proto: None,
        epoch,
    });
}

fn prepare_slice(picker: &Picker, request: &SliceRequest) -> Option<StatefulProtocol> {
    let crop = request.image.crop_imm(
        0,
        request.y0,
        request.image.width(),
        request.y1 - request.y0,
    );
    let font = picker.font_size();
    let width = u32::from(request.size.width.max(1)) * u32::from(font.width.max(1));
    let height = u32::from(font.height.max(1));
    let scaled = crop.resize_exact(width, height, FilterType::Nearest);
    let mut proto = picker.new_resize_protocol(scaled);
    proto.resize_encode(&Resize::Fit(None), request.size);
    proto.last_encoding_result()?.ok()?;
    Some(proto)
}

/// Max decoded image dimension (px per side) and total allocation.
const MAX_IMAGE_DIM: u32 = 8000;
const MAX_IMAGE_ALLOC: u64 = 256 * 1024 * 1024;
const MAX_SLICE_PROTOS: usize = 512;
const MAX_SLICE_WORKERS: usize = 4;
const GENERATED_KEY_PREFIX: &str = "\u{0}";

/// Fetch a remote image's bytes, reusing the PDF pipeline's downloader.
/// (SVG bytes won't decode as a raster — those stay placeholders for now.)
#[cfg(not(target_arch = "wasm32"))]
fn fetch_remote(url: &str) -> Option<Vec<u8>> {
    crate::render::image::fetch_remote_image(url)
        .ok()
        .map(|(bytes, _ext)| bytes)
}

#[cfg(target_arch = "wasm32")]
fn fetch_remote(_url: &str) -> Option<Vec<u8>> {
    None
}

/// Resolve a local image path, jailed to the document's directory.
///
/// Untrusted documents must not read arbitrary files, so absolute paths are
/// rejected and the canonicalized target (which resolves `..` and symlinks)
/// must stay within the canonicalized base directory.
fn resolve(src: &str, base: Option<&Path>) -> Option<PathBuf> {
    if Path::new(src).is_absolute() {
        return None;
    }
    let canon_base = base?.canonicalize().ok()?;
    let candidate = canon_base.join(src).canonicalize().ok()?;
    candidate.starts_with(&canon_base).then_some(candidate)
}

/// Reserve the number of rows the image will actually occupy: its natural cell
/// size (pixels / `cell`), downscaled to fit `content_width` (never upscaled),
/// matching how ratatui-image's `Fit` renders. Bounded by `max_rows` so a band
/// cannot flood the content flow; over-tall bands are scrolled through by
/// cropping the visible source slice during draw.
pub(super) fn reserved_rows(
    width: u32,
    height: u32,
    content_width: u16,
    cell: (u32, u32),
    max_rows: u16,
) -> u16 {
    let (cell_w, cell_h) = (cell.0.max(1), cell.1.max(1));
    let natural_cols = width.max(1).div_ceil(cell_w).max(1);
    let natural_rows = height.max(1).div_ceil(cell_h).max(1);
    let limit = u32::from(content_width).max(1);
    let rows = if natural_cols <= limit {
        natural_rows
    } else {
        (natural_rows * limit / natural_cols).max(1)
    };
    let cap = u32::from(max_rows.max(1));
    u16::try_from(rows.min(cap)).unwrap_or(max_rows).max(1)
}

fn slice_bounds(height: u32, start_row: u16, rows: u16, band_rows: u16) -> Option<(u32, u32)> {
    if height == 0 || rows == 0 || band_rows == 0 {
        return None;
    }

    let band = u32::from(band_rows);
    let start = u32::from(start_row);
    if start >= band {
        return None;
    }

    let end = start.saturating_add(u32::from(rows)).min(band);
    if end <= start {
        return None;
    }

    let h = u64::from(height);
    let band_u64 = u64::from(band);
    let y0 = u32::try_from(u64::from(start) * h / band_u64)
        .ok()?
        .min(height - 1);
    let y1 = u32::try_from((u64::from(end) * h).div_ceil(band_u64))
        .ok()?
        .clamp(y0 + 1, height);
    Some((y0, y1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use std::time::Duration;

    fn test_image(width: u32, height: u32) -> DynamicImage {
        ImageBuffer::from_pixel(width, height, Rgba([255_u8, 0, 128, 255])).into()
    }

    fn test_request(src: &str, row: u16) -> SliceRequest {
        test_request_at(src, 0, row)
    }

    fn test_request_at(src: &str, placement_line: u16, row: u16) -> SliceRequest {
        SliceRequest {
            key: SliceKey {
                src: src.to_string(),
                placement_line,
                row,
                band_rows: 10,
                area_width: 20,
                generation: 0,
            },
            image: Arc::new(test_image(80, 80)),
            y0: 0,
            y1: 8,
            size: Size::new(20, 1),
            epoch: 0,
        }
    }

    fn wait_for_ready(store: &mut ImageStore) {
        for _ in 0..100 {
            if store.poll_ready() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for async image slice");
    }

    #[test]
    fn reserved_rows_use_natural_size_then_downscale() {
        let cell = (8, 16);
        // 100x100 px at 8x16 cells → ~13 cols x 7 rows; fits in 40 cols → 7 rows.
        assert_eq!(reserved_rows(100, 100, 40, cell, 50), 7);
        // Wide banner (1000x100 → 125 cols) exceeds 60 → downscaled to a few rows.
        assert!(reserved_rows(1000, 100, 60, cell, 50) <= 4);
        // Never zero.
        assert!(reserved_rows(100, 1, 80, cell, 50) >= 1);
        // A tall diagram is clamped to the configured band cap, not the viewport.
        assert_eq!(reserved_rows(400, 4000, 80, cell, 30), 30);
    }

    #[test]
    fn slice_bounds_map_band_rows_to_pixels() {
        assert_eq!(slice_bounds(100, 0, 10, 100), Some((0, 10)));
        assert_eq!(slice_bounds(100, 90, 10, 100), Some((90, 100)));
        assert_eq!(slice_bounds(101, 1, 1, 3), Some((33, 68)));
    }

    #[test]
    fn slice_bounds_reject_bad_ranges_without_panicking() {
        assert_eq!(slice_bounds(100, 100, 1, 100), None);
        assert_eq!(slice_bounds(100, 0, 0, 100), None);
        assert_eq!(slice_bounds(100, 0, 1, 0), None);
        assert_eq!(slice_bounds(0, 0, 1, 100), None);
    }

    #[test]
    fn resolve_rejects_missing() {
        assert!(resolve("does-not-exist.png", Some(Path::new("/tmp"))).is_none());
    }

    #[test]
    fn row_protocol_prepares_missing_row_asynchronously() {
        let mut store = ImageStore::new(Some(Picker::halfblocks()), None);
        store.ensure_generated("generated", || Some(test_image(80, 80)));
        let area = Rect::new(0, 0, 20, 1);

        assert!(store.row_protocol("generated", 0, 0, 10, area).is_none());
        assert!(store.has_pending());

        wait_for_ready(&mut store);

        assert!(!store.has_pending());
        assert!(store.row_protocol("generated", 0, 0, 10, area).is_some());
    }

    #[test]
    fn row_protocol_does_not_substitute_missing_rows() {
        let mut store = ImageStore::new(Some(Picker::halfblocks()), None);
        store.ensure_generated("generated", || Some(test_image(80, 80)));
        let area = Rect::new(0, 0, 20, 1);

        assert!(store.row_protocol("generated", 0, 0, 10, area).is_none());
        wait_for_ready(&mut store);
        assert!(store.row_protocol("generated", 0, 0, 10, area).is_some());

        assert!(store.row_protocol("generated", 0, 1, 10, area).is_none());
        assert!(store.has_pending());
    }

    #[test]
    fn prefetch_row_prepares_without_marking_visible() {
        let mut store = ImageStore::new(Some(Picker::halfblocks()), None);
        store.ensure_generated("generated", || Some(test_image(80, 80)));
        let area = Rect::new(0, 0, 20, 1);

        store.prefetch_row("generated", 0, 1, 10, area.width);

        assert!(store.has_pending());
        assert!(store.visible.is_empty());
        assert_eq!(store.wanted.len(), 1);
    }

    #[test]
    fn warm_rows_prepares_without_marking_wanted_or_visible() {
        let mut store = ImageStore::new(Some(Picker::halfblocks()), None);
        store.ensure_generated("generated", || Some(test_image(80, 80)));

        store.warm_rows("generated", 0, 0, 3, 10, 20);

        assert!(store.has_pending());
        assert!(store.wanted.is_empty());
        assert!(store.visible.is_empty());
        assert_eq!(store.warming.len(), 3);
    }

    #[test]
    fn prefetched_ready_row_does_not_request_redraw_until_visible() {
        let (tx, _rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let mut store = ImageStore::new(Some(Picker::halfblocks()), None);
        store.ensure_generated("generated", || Some(test_image(80, 80)));
        store.worker = Some(SliceWorker {
            txs: vec![tx],
            rx: ready_rx,
        });
        let request = store
            .build_request("generated", 0, 1, 10, 20)
            .expect("request");
        let proto = prepare_slice(&Picker::halfblocks(), &request);
        store.wanted.insert(request.key.clone());
        store.pending.insert(request.key.clone(), request.epoch);
        ready_tx
            .send(SliceReady {
                key: request.key.clone(),
                proto,
                epoch: request.epoch,
            })
            .expect("ready");

        assert!(!store.poll_ready());
        assert!(store.slices.contains_key(&request.key));
    }

    #[test]
    fn warmed_ready_row_is_cached_even_after_epoch_changes() {
        let (tx, _rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let mut store = ImageStore::new(Some(Picker::halfblocks()), None);
        store.ensure_generated("generated", || Some(test_image(80, 80)));
        store.worker = Some(SliceWorker {
            txs: vec![tx],
            rx: ready_rx,
        });
        let request = store
            .build_request("generated", 0, 1, 10, 20)
            .expect("request");
        let proto = prepare_slice(&Picker::halfblocks(), &request);
        store.warming.insert(request.key.clone());
        store.pending.insert(request.key.clone(), request.epoch);
        store.begin_frame(ImageView {
            scroll: 1,
            height: 10,
            width: 20,
        });
        ready_tx
            .send(SliceReady {
                key: request.key.clone(),
                proto,
                epoch: request.epoch,
            })
            .expect("ready");

        assert!(!store.poll_ready());
        assert!(store.slices.contains_key(&request.key));
        assert!(!store.warming.contains(&request.key));
    }

    #[test]
    fn slice_worker_keeps_different_sources_queued() {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let mut queued = VecDeque::new();
        let mut current = test_request("a", 0);
        let mut min_epoch = 0;
        let mut canceled = HashSet::new();

        tx.send(SliceMsg::Request(test_request("b", 0)))
            .expect("send");
        drain_slice_requests(
            &rx,
            &ready_tx,
            &mut queued,
            &mut current,
            &mut min_epoch,
            &mut canceled,
        );

        assert_eq!(current.key.src, "a");
        assert_eq!(queued.len(), 1);
        assert_eq!(queued.front().expect("queued").key.src, "b");
        assert!(ready_rx.try_recv().is_err());
    }

    #[test]
    fn slice_worker_replaces_duplicate_row_request() {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let mut queued = VecDeque::new();
        let mut current = test_request("a", 0);
        let mut min_epoch = 0;
        let mut canceled = HashSet::new();

        tx.send(SliceMsg::Request(test_request("a", 0)))
            .expect("send");
        drain_slice_requests(
            &rx,
            &ready_tx,
            &mut queued,
            &mut current,
            &mut min_epoch,
            &mut canceled,
        );

        assert_eq!(current.key.src, "a");
        assert_eq!(current.key.row, 0);
        assert!(queued.is_empty());
        let canceled = ready_rx.try_recv().expect("canceled");
        assert_eq!(canceled.key.row, 0);
        assert!(canceled.proto.is_none());
    }

    #[test]
    fn slice_worker_keeps_same_source_different_rows_queued() {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let mut queued = VecDeque::new();
        let mut current = test_request("a", 0);
        let mut min_epoch = 0;
        let mut canceled = HashSet::new();

        tx.send(SliceMsg::Request(test_request("a", 2)))
            .expect("send");
        drain_slice_requests(
            &rx,
            &ready_tx,
            &mut queued,
            &mut current,
            &mut min_epoch,
            &mut canceled,
        );

        assert_eq!(current.key.row, 0);
        assert_eq!(queued.len(), 1);
        assert_eq!(queued.front().expect("queued").key.row, 2);
        assert!(ready_rx.try_recv().is_err());
    }

    #[test]
    fn slice_worker_keeps_same_source_at_different_placements_queued() {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let mut queued = VecDeque::new();
        let mut current = test_request_at("a", 10, 0);
        let mut min_epoch = 0;
        let mut canceled = HashSet::new();

        tx.send(SliceMsg::Request(test_request_at("a", 40, 2)))
            .expect("send");
        drain_slice_requests(
            &rx,
            &ready_tx,
            &mut queued,
            &mut current,
            &mut min_epoch,
            &mut canceled,
        );

        assert_eq!(current.key.placement_line, 10);
        assert_eq!(queued.len(), 1);
        assert_eq!(queued.front().expect("queued").key.placement_line, 40);
        assert!(ready_rx.try_recv().is_err());
    }

    #[test]
    fn finish_frame_cancels_pending_rows_that_left_the_view() {
        let (tx, rx) = mpsc::channel();
        let (_ready_tx, ready_rx) = mpsc::channel();
        let mut store = ImageStore::new(None, None);
        store.worker = Some(SliceWorker {
            txs: vec![tx],
            rx: ready_rx,
        });
        let request = test_request("a", 0);
        store.pending.insert(request.key.clone(), request.epoch);

        store.finish_frame();

        assert!(store.pending.is_empty());
        let SliceMsg::Cancel { key, epoch } = rx.try_recv().expect("cancel") else {
            panic!("expected cancel message");
        };
        assert_eq!(key, request.key);
        assert_eq!(epoch, request.epoch);
    }

    #[test]
    fn finish_frame_keeps_pending_rows_that_are_still_visible() {
        let (tx, rx) = mpsc::channel();
        let (_ready_tx, ready_rx) = mpsc::channel();
        let mut store = ImageStore::new(None, None);
        store.worker = Some(SliceWorker {
            txs: vec![tx],
            rx: ready_rx,
        });
        let request = test_request("a", 0);
        store.wanted.insert(request.key.clone());
        store.pending.insert(request.key.clone(), request.epoch);

        store.finish_frame();

        assert_eq!(store.pending.get(&request.key), Some(&request.epoch));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn finish_frame_keeps_background_warming_rows() {
        let (tx, rx) = mpsc::channel();
        let (_ready_tx, ready_rx) = mpsc::channel();
        let mut store = ImageStore::new(None, None);
        store.worker = Some(SliceWorker {
            txs: vec![tx],
            rx: ready_rx,
        });
        let request = test_request("a", 0);
        store.warming.insert(request.key.clone());
        store.pending.insert(request.key.clone(), request.epoch);

        store.finish_frame();

        assert_eq!(store.pending.get(&request.key), Some(&request.epoch));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn slice_worker_ignores_stale_cancel_for_already_finished_row() {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let mut queued = VecDeque::new();
        let mut current = test_request("b", 0);
        let old = test_request("a", 0);
        let mut min_epoch = 0;
        let mut canceled = HashSet::new();

        tx.send(SliceMsg::Cancel {
            key: old.key.clone(),
            epoch: old.epoch,
        })
        .expect("send");
        drain_slice_requests(
            &rx,
            &ready_tx,
            &mut queued,
            &mut current,
            &mut min_epoch,
            &mut canceled,
        );

        assert!(canceled.is_empty());
        assert!(ready_rx.try_recv().is_err());
    }

    #[test]
    fn prepared_slice_does_not_resize_again_for_render_area() {
        let picker = Picker::halfblocks();
        let request = test_request("a", 0);
        let proto = prepare_slice(&picker, &request).expect("prepared");

        assert!(
            proto
                .needs_resize(&Resize::Fit(None), request.size)
                .is_none(),
            "worker result should be ready for StatefulImage render"
        );
    }
}
