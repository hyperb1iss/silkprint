use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use image::{DynamicImage, imageops::FilterType};
use ratatui::layout::Size;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{Resize, ResizeEncodeRender};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct SliceKey {
    pub(super) src: String,
    pub(super) placement_line: u16,
    pub(super) row: u16,
    pub(super) band_rows: u16,
    pub(super) area_width: u16,
    pub(super) generation: u64,
}

pub(super) struct SliceRequest {
    pub(super) key: SliceKey,
    pub(super) image: Arc<DynamicImage>,
    pub(super) y0: u32,
    pub(super) y1: u32,
    pub(super) size: Size,
    pub(super) epoch: u64,
}

pub(super) struct SliceReady {
    pub(super) key: SliceKey,
    pub(super) proto: Option<StatefulProtocol>,
    pub(super) epoch: u64,
}

pub(super) enum SliceMsg {
    Request(SliceRequest),
    Cancel { key: SliceKey, epoch: u64 },
    CancelBefore(u64),
}

pub(super) struct SliceWorker {
    pub(super) txs: Vec<Sender<SliceMsg>>,
    pub(super) rx: Receiver<SliceReady>,
}

impl SliceWorker {
    pub(super) fn send(&self, request: SliceRequest) -> Result<(), mpsc::SendError<SliceRequest>> {
        let idx = request.key.worker_index(self.txs.len());
        self.txs[idx]
            .send(SliceMsg::Request(request))
            .map_err(|err| match err.0 {
                SliceMsg::Request(request) => mpsc::SendError(request),
                SliceMsg::Cancel { .. } | SliceMsg::CancelBefore(_) => unreachable!("sent request"),
            })
    }

    pub(super) fn cancel_before(&self, epoch: u64) {
        for tx in &self.txs {
            let _ = tx.send(SliceMsg::CancelBefore(epoch));
        }
    }

    pub(super) fn cancel(&self, key: SliceKey, epoch: u64) {
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

pub(super) fn spawn_slice_workers(picker: &Picker) -> SliceWorker {
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

pub(super) fn drain_slice_requests(
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

pub(super) fn prepare_slice(picker: &Picker, request: &SliceRequest) -> Option<StatefulProtocol> {
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

const MAX_SLICE_WORKERS: usize = 4;
